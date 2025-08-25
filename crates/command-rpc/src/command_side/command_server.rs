use anyhow::Context;
use flow_lib::command::CommandFactory;
use iroh::Watcher;
use iroh::{Endpoint, NodeAddr};
use rand::rngs::OsRng;
use serde::Deserialize;
use serde_with::DisplayFromStr;
use std::{collections::BTreeSet, net::SocketAddr, time::Duration};
use url::Url;

use crate::flow_side::address_book::{self, AddressBookExt};
use crate::tracing::TrackFlowRun;

use super::{
    command_factory::{self, CommandFactoryExt},
    command_trait::HTTP_CLIENT,
};

#[derive(Deserialize)]
#[serde(untagged)]
pub enum FlowServerConfig {
    Info { url: Url },
    Direct(FlowServerAddress),
}

#[serde_with::serde_as]
#[derive(Deserialize)]
pub struct Config {
    pub flow_server: FlowServerConfig,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub secret_key: Option<iroh::SecretKey>,
    pub apikey: Option<String>,
}

#[derive(Deserialize)]
pub struct FlowServerAddress {
    pub node_id: iroh::PublicKey,
    pub relay_url: Url,
    pub direct_addresses: BTreeSet<SocketAddr>,
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

pub async fn serve(config: Config, logs: TrackFlowRun) -> Result<(), anyhow::Error> {
    let server = match config.flow_server {
        FlowServerConfig::Info { url } => {
            let info_url = url.join("/info")?;
            reqwest::get(info_url)
                .await?
                .json::<InfoResponse>()
                .await?
                .iroh
        }
        FlowServerConfig::Direct(server) => server,
    };

    let servers = [server];

    let factory = CommandFactory::collect();
    let availables = factory.availables().collect::<Vec<_>>();

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
    let direct_addresses: BTreeSet<SocketAddr> = endpoint
        .direct_addresses()
        .initialized()
        .await?
        .into_iter()
        .map(|addr| addr.addr)
        .collect();
    let relay_url: Url = endpoint.home_relay().initialized().await?.into();
    let mut clients = Vec::new();
    for addr in &servers {
        let client = address_book::connect_iroh(
            endpoint.clone(),
            NodeAddr {
                node_id: addr.node_id,
                relay_url: Some(addr.relay_url.clone().into()),
                direct_addresses: addr.direct_addresses.clone(),
            },
        )
        .await?;
        client
            .join(
                direct_addresses.clone(),
                relay_url.clone(),
                &availables,
                config.apikey.clone(),
            )
            .await?;
        clients.push(client);
        tracing::info!("joined {}", addr.node_id);
        let conn_type = endpoint
            .conn_type(addr.node_id)
            .and_then(|watcher| watcher.get().ok());
        tracing::info!("connection type {:?}", conn_type);
    }

    let client = command_factory::new_client(factory, logs);
    client.bind_iroh(endpoint);

    tokio::signal::ctrl_c().await.ok();

    tokio::time::timeout(Duration::from_secs(5), async {
        for client in clients {
            client.leave().await?;
            tracing::info!("left");
        }
        Ok(())
    })
    .await?
}
