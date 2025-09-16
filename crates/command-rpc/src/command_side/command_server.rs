use anyhow::Context;
use flow_lib::command::{CommandFactory, MatchCommand};
use futures::TryFutureExt;
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
            address: FlowServerAddressConfig::Info {
                url: "https://dev-api.spaceoperator.com".parse().unwrap(),
            },
        }
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum FlowServerAddressConfig {
    Info { url: Url },
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

pub fn main() {
    let tracker = TrackFlowRun::init_tracing();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    // initializing these clients take a long time
    let _ = *HTTP_CLIENT;
    rt.block_on(async {
        let data = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
        let config: Config = toml::from_str(&data).unwrap();
        tokio::task::LocalSet::new()
            .run_until(serve(config, tracker))
            .await
            .unwrap();
    })
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
        .await?
        .into_iter()
        .map(|addr| addr.addr)
        .collect();
    let relay_url: Url = endpoint.home_relay().initialized().await?.into();

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
        .and_then(|watcher| watcher.get().ok());
    tracing::info!("connection type {:?}", conn_type);

    cancel.cancelled().await;

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

    tokio::signal::ctrl_c().await.ok();
    cancel.cancel();

    join_set.join_all().await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use schemars::schema_for;

    use super::*;
    #[test]
    fn generate_schema() {
        println!(
            "{}",
            serde_json::to_string_pretty(&schema_for!(FlowServerConfig)).unwrap()
        );
    }
}
