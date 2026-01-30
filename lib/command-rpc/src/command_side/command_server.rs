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

#[derive(Deserialize, schemars::JsonSchema)]
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

#[derive(Deserialize, schemars::JsonSchema)]
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

#[derive(Deserialize, schemars::JsonSchema)]
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

async fn ping(client: &address_book::Client) {
    loop {
        if let Err(error) = client.ping().await {
            tracing::error!("ping failed: {:#}", error);
            break;
        }
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

pub async fn serve_server(
    endpoint: Endpoint,
    config: FlowServerConfig,
    availables: Vec<MatchCommand>,
    cancel: CancellationToken,
) -> Result<(), anyhow::Error> {
    let server_addr = match config.address {
        FlowServerAddressConfig::Info { url } => {
            let info_url = url.join("/info")?;
            reqwest::get(info_url)
                .await?
                .json::<InfoResponse>()
                .await?
                .iroh
        }
        FlowServerAddressConfig::Direct(server) => server,
    };

    let direct_addresses: BTreeSet<SocketAddr> = endpoint
        .direct_addresses()
        .initialized()
        .await
        .into_iter()
        .map(|addr| addr.addr)
        .collect();
    let relay_url: Url = endpoint.home_relay().initialized().await.into();

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

    future::select(
        std::pin::pin!(ping(&client)),
        std::pin::pin!(cancel.cancelled()),
    )
    .await;

    const LEAVE_TIMEOUT: Duration = Duration::from_secs(3);
    let _ = tokio::time::timeout(LEAVE_TIMEOUT, client.leave()).await;
    tracing::info!("left {}", server_addr.node_id);

    Ok(())
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
