use anyhow::{Context, anyhow};
use flow_lib::command::{CommandFactory, MatchCommand};
use futures::{TryFutureExt, future};
use iroh::Watcher;
use iroh::{Endpoint, NodeAddr};
use rand::rngs::OsRng;
use serde::Deserialize;
use serde_with::DisplayFromStr;
use std::{collections::BTreeSet, net::SocketAddr, time::Duration};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::flow_side::address_book::{self, AddressBookExt};
use crate::tracing::TrackFlowRun;

use super::{
    command_factory::{self, CommandFactoryExt},
    command_trait::HTTP_CLIENT,
};

#[derive(Clone, Deserialize, schemars::JsonSchema)]
pub struct FlowServerConfig {
    apikey: Option<String>,
    #[serde(flatten)]
    address: FlowServerAddressConfig,
}

impl Default for FlowServerConfig {
    fn default() -> Self {
        Self {
            apikey: None,
            address: FlowServerAddressConfig::Info { url: default_url() },
        }
    }
}

fn default_url() -> Url {
    "https://dev-api.spaceoperator.com".parse().unwrap()
}

#[derive(Clone, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum FlowServerAddressConfig {
    Info {
        #[schemars(default = "default_url")]
        #[schemars(example = "https://dev-api.spaceoperator.com/")]
        #[schemars(example = "http://localhost:8080/")]
        url: Url,
    },
    Direct(FlowServerAddress),
}

fn default_flow_server() -> Vec<FlowServerConfig> {
    [FlowServerConfig::default()].into()
}

#[serde_with::serde_as]
#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default_flow_server")]
    pub flow_server: Vec<FlowServerConfig>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub secret_key: Option<iroh::SecretKey>,
    pub apikey: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, schemars::JsonSchema)]
struct ConfigSchema {
    #[serde(default = "default_flow_server")]
    flow_server: Vec<FlowServerConfig>,
    #[schemars(schema_with = "Option::<String>::json_schema")]
    secret_key: Option<iroh::SecretKey>,
    apikey: Option<String>,
}

#[derive(Clone, Deserialize, schemars::JsonSchema)]
pub struct FlowServerAddress {
    #[schemars(schema_with = "String::json_schema")]
    pub node_id: iroh::PublicKey,
    pub relay_url: Url,
    pub direct_addresses: Option<BTreeSet<SocketAddr>>,
}

#[derive(Deserialize)]
struct InfoResponse {
    iroh: FlowServerAddress,
}

pub fn main() -> Result<(), anyhow::Error> {
    let tracker = TrackFlowRun::init_tracing();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    // initializing these clients take a long time
    let _ = *HTTP_CLIENT;
    rt.block_on(async {
        let path = std::env::args().nth(1).unwrap();
        let data =
            std::fs::read_to_string(&path).with_context(|| format!("error reading {path}"))?;
        let config: Config = serde_json::from_value(
            jsonc_parser::parse_to_serde_value(&data, &<_>::default())?.unwrap_or_default(),
        )?;
        tokio::task::LocalSet::new()
            .run_until(serve(config, tracker))
            .await?;
        Ok(())
    })
}

async fn ping(client: &address_book::Client) -> Result<(), anyhow::Error> {
    loop {
        if let Err(error) = client.ping().await {
            tracing::error!("ping failed: {:#}", error);
            return Err(error);
        }
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

async fn sleep_or_cancel(duration: Duration, cancel: &CancellationToken) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(duration) => false,
        _ = cancel.cancelled() => true,
    }
}

async fn serve_server_once(
    endpoint: Endpoint,
    config: FlowServerConfig,
    availables: Vec<MatchCommand>,
    cancel: CancellationToken,
) -> Result<(), anyhow::Error> {
    const INFO_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
    const IROH_INFO_TIMEOUT: Duration = Duration::from_secs(10);

    let server_addr = match config.address {
        FlowServerAddressConfig::Info { url } => {
            let info_url = url.join("/info")?;
            tracing::info!("using URL: {}", info_url);
            let resp = tokio::time::timeout(INFO_REQUEST_TIMEOUT, async {
                HTTP_CLIENT
                    .get(info_url.clone())
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<InfoResponse>()
                    .await
            })
            .await
            .context("timed out fetching flow-server /info")?
            .context("fetch flow-server /info")?
            .iroh;
            resp
        }
        FlowServerAddressConfig::Direct(server) => server,
    };

    let mut direct_addresses = endpoint.direct_addresses();
    let mut home_relay = endpoint.home_relay();
    let (direct_addresses, relay_url) = tokio::time::timeout(IROH_INFO_TIMEOUT, async {
        tokio::join!(direct_addresses.initialized(), home_relay.initialized())
    })
    .await
    .context("timed out waiting for local iroh endpoint info")?;
    let direct_addresses: BTreeSet<SocketAddr> =
        direct_addresses.into_iter().map(|addr| addr.addr).collect();
    let relay_url: Url = relay_url.into();

    let client = address_book::connect_iroh(
        endpoint.clone(),
        NodeAddr {
            node_id: server_addr.node_id,
            relay_url: Some(server_addr.relay_url.clone().into()),
            direct_addresses: server_addr.direct_addresses.clone().unwrap_or_default(),
        },
    )
    .await?;
    client
        .join(
            direct_addresses.clone(),
            relay_url.clone(),
            &availables,
            config.apikey,
        )
        .await?;
    tracing::info!("joined {}", server_addr.node_id);
    let conn_type = endpoint
        .conn_type(server_addr.node_id)
        .map(|mut watcher| watcher.get());
    tracing::info!("connection type {:?}", conn_type);

    let ping_result = tokio::select! {
        result = ping(&client) => result,
        _ = cancel.cancelled() => Ok(()),
    };

    const LEAVE_TIMEOUT: Duration = Duration::from_secs(3);
    let _ = tokio::time::timeout(LEAVE_TIMEOUT, client.leave()).await;
    tracing::info!("left {}", server_addr.node_id);

    ping_result?;
    Ok(())
}

pub async fn serve_server(
    endpoint: Endpoint,
    config: FlowServerConfig,
    availables: Vec<MatchCommand>,
    cancel: CancellationToken,
) -> Result<(), anyhow::Error> {
    const RETRY_DELAY: Duration = Duration::from_secs(30);

    loop {
        match serve_server_once(
            endpoint.clone(),
            FlowServerConfig {
                apikey: config.apikey.clone(),
                address: config.address.clone(),
            },
            availables.clone(),
            cancel.clone(),
        )
        .await
        {
            Ok(()) => return Ok(()),
            Err(error) if cancel.is_cancelled() => return Err(error),
            Err(error) => {
                tracing::error!("command server connection failed: {error:#}; retrying");
                if sleep_or_cancel(RETRY_DELAY, &cancel).await {
                    return Ok(());
                }
            }
        }
    }
}

pub async fn serve(config: Config, logs: TrackFlowRun) -> Result<(), anyhow::Error> {
    let endpoint = Endpoint::builder()
        .secret_key(
            config
                .secret_key
                .unwrap_or_else(|| iroh::SecretKey::generate(&mut OsRng)),
        )
        .discovery_n0()
        .bind()
        .await
        .context("bind iroh endpoint")?;
    tracing::info!("using public key: {}", endpoint.node_id());
    let factory = CommandFactory::collect();
    let availables = factory.availables().collect::<Vec<_>>();
    command_factory::new_client(factory, logs).bind_iroh(endpoint.clone());

    let cancel = CancellationToken::new();

    let mut join_set = JoinSet::new();
    for mut server_config in config.flow_server {
        server_config.apikey = server_config.apikey.or_else(|| config.apikey.clone());
        join_set.spawn_local(
            serve_server(
                endpoint.clone(),
                server_config,
                availables.clone(),
                cancel.clone(),
            )
            .inspect_err(|error| tracing::error!("error: {error}")),
        );
    }

    match future::select(
        std::pin::pin!(join_set.join_all()),
        std::pin::pin!(tokio::signal::ctrl_c()),
    )
    .await
    {
        future::Either::Left((results, _)) => {
            if results.iter().all(|r| r.is_err()) {
                Err(anyhow!("all connects failed"))
            } else {
                Ok(())
            }
        }
        future::Either::Right((_, join)) => {
            cancel.cancel();
            join.await;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use schemars::schema_for;

    use super::*;
    #[test]
    fn generate_schema() {
        let serde_json::Value::Object(mut schema) =
            serde_json::to_value(&schema_for!(ConfigSchema)).unwrap()
        else {
            panic!()
        };
        schema["$schema"] = "http://json-schema.org/draft-07/schema#".into();
        schema.shift_insert(
            1,
            "id".into(),
            "https://schema.spaceoperator.com/command-server-config.schema.json".into(),
        );
        let text = serde_json::to_string_pretty(&schema).unwrap();
        std::fs::write("../../schema/command-server-config.schema.json", &text).unwrap();
    }
}
