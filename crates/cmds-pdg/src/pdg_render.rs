use flow_lib::command::prelude::*;
use futures::{FutureExt, SinkExt, StreamExt, stream::BoxStream};
use once_cell::sync::Lazy;
use pdg_common::{PostReply, RenderRequest, RenderSuccess, ResultBool, WaitRequest, WorkItem};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, future::pending, time::Duration};
use thiserror::Error as ThisError;
use tokio::{net::TcpStream, time::Instant};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Error as WsError, Message},
};
use tracing::instrument::WithSubscriber;
use uuid::Uuid;

const PDG_RENDER: &str = "pdg_render";

fn build() -> BuildResult {
    static CACHE: Lazy<Result<CmdBuilder, BuilderError>> = Lazy::new(|| {
        CmdBuilder::new(flow_lib::node_definition!("pdg_render.json"))?.check_name(PDG_RENDER)
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(PDG_RENDER, |_| build()));

fn default_url() -> String {
    // "ws://127.0.0.1:8081/render".to_owned()
    "wss://dev-api.spaceoperator.com/pdg/render".to_owned()
}

#[derive(Serialize, Deserialize, Debug)]
struct Input {
    #[serde(default = "default_url")]
    url: String,
    rand_seed: Option<String>,
    #[serde(default)]
    attributes: HashMap<String, serde_json::Value>,
    #[serde(default)]
    headers: HashMap<String, String>,
}

#[derive(Serialize, Debug)]
struct Output {
    main_image_url: String,
    sketch_image_url: String,
    metadata_url: String,
    metadata: flow_lib::Value,
}

#[derive(ThisError, Debug)]
#[error("server disconnected")]
struct Disconnected;

fn run_ws(
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
) -> BoxStream<'static, Result<String, WsError>> {
    // ping every 30 secs
    const PING_INTERVAL: Duration = Duration::from_secs(30);
    // if server doesn't pong in 8 secs, consider it dead
    const PONG_TIMEOUT: Duration = Duration::from_secs(8);

    let (text_tx, text_rx) = futures::channel::mpsc::unbounded();
    let (mut write, mut read) = ws.split();
    tokio::spawn(
        async move {
            let mut pong_deadline = pending::<()>().boxed();
            let mut ping_interval =
                tokio::time::interval_at(Instant::now() + PING_INTERVAL, PING_INTERVAL);
            ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select!(
                    _ = ping_interval.tick() => {
                        if let Err(error) = write.send(Message::Ping(Vec::new())).await {
                            text_tx.unbounded_send(Err(error)).ok();
                            break;
                        }
                        pong_deadline = Box::pin(tokio::time::sleep_until(Instant::now() + PONG_TIMEOUT));
                    },
                    _ = &mut pong_deadline => {
                        text_tx.unbounded_send(Err(WsError::ConnectionClosed)).ok();
                        break;
                    },
                    res = read.next() => {
                        match res {
                            Some(Ok(msg)) => {
                                tracing::trace!("received message: {:?}", msg);
                                pong_deadline = pending::<()>().boxed();
                                if let Message::Text(text) = msg
                                    && text_tx.unbounded_send(Ok(text)).is_err() {
                                        break;
                                    }
                            }
                            Some(Err(error)) => {
                                text_tx.unbounded_send(Err(error)).ok();
                                break;
                            }
                            None => {
                                break;
                            }
                        }
                    }
                );
            }
        }
        .with_current_subscriber()
    );

    text_rx.boxed()
}

async fn ws_wait(
    render_url: &str,
    request_uuid: Uuid,
) -> Result<BoxStream<'_, Result<String, WsError>>, CommandError> {
    let url = match render_url.strip_suffix("/render") {
        Some(s) => format!("{s}/wait"),
        None => return Err(CommandError::msg("could not build URL for waiting")),
    };
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await?;
    ws.send(serde_json::to_string(&WaitRequest { request_uuid })?.into())
        .await?;

    Ok(run_ws(ws))
}

async fn run(_: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (mut ws, _) = tokio_tungstenite::connect_async(&input.url).await?;

    let rand_seed = input.rand_seed.or_else(|| {
        Some(
            input
                .attributes
                .get("wedgeindex")?
                .pointer("/value/0")?
                .as_i64()?
                .to_string(),
        )
    });

    // send the request
    ws.send({
        tracing::debug!(
            "rand_seed={}",
            &rand_seed.as_ref().unwrap_or(&"".to_owned())
        );
        let text = serde_json::to_string(&RenderRequest {
            rand_seed,
            version: "6".to_owned(),
            workitem: WorkItem {
                attributes: input.attributes,
                ..<_>::default()
            },
        })?;
        tracing::debug!("{}", text);
        text.into()
    })
    .await?;

    let mut text_stream = run_ws(ws);

    let id = serde_json::from_str::<ResultBool<PostReply>>(
        &text_stream.next().await.ok_or(Disconnected)??,
    )?
    .into_result()??
    .request_uuid;
    tracing::info!("request_uuid={}", id);

    let mut tries = 10;

    let res = loop {
        let text = match text_stream.next().await {
            Some(Ok(text)) => text,
            Some(Err(error)) => {
                tracing::warn!("error: {}, reconnecting", error);
                tries -= 1;
                if tries == 0 {
                    return Err(CommandError::msg("too many errors"));
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
                text_stream = ws_wait(&input.url, id).await?;
                continue;
            }
            None => {
                tracing::warn!("connection closed, reconnecting");
                tries -= 1;
                if tries == 0 {
                    return Err(CommandError::msg("too many errors"));
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
                text_stream = ws_wait(&input.url, id).await?;
                continue;
            }
        };
        let res = serde_json::from_str::<ResultBool<RenderSuccess>>(&text)?.into_result()??;
        break res;
    };

    let metadata = reqwest::get(&res.metadata_url)
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(Output {
        main_image_url: res.main_image_url,
        sketch_image_url: res.sketch_image_url,
        metadata_url: res.metadata_url,
        metadata: metadata.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
