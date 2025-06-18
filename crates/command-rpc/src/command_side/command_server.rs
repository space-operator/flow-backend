use anyhow::Context;
use flow_lib::command::collect_commands;
use iroh::{Endpoint, NodeAddr};
use serde::Deserialize;
use serde_with::DisplayFromStr;
use std::{collections::BTreeSet, net::SocketAddr};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use url::Url;

use crate::flow_side::address_book::{self, AddressBookExt};

use super::command_factory::{self, CommandFactoryExt};

#[serde_with::serde_as]
#[derive(Deserialize)]
pub struct Config {
    pub flow_server_url: Url,
    #[serde_as(as = "DisplayFromStr")]
    pub secret_key: iroh::SecretKey,
}

#[derive(Deserialize)]
pub struct FlowServerAddress {
    pub node_id: iroh::PublicKey,
    pub direct_addresses: BTreeSet<SocketAddr>,
    pub relay_url: Url,
}

#[derive(Deserialize)]
struct InfoResponse {
    iroh: FlowServerAddress,
}

pub fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let data = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
        let config: Config = toml::from_str(&data).unwrap();
        let local = tokio::task::LocalSet::new();
        local.run_until(serve(config)).await.unwrap();
    })
}

pub async fn serve(config: Config) -> Result<(), anyhow::Error> {
    let info_url = config.flow_server_url.join("/info")?;
    let server = reqwest::get(info_url)
        .await?
        .json::<InfoResponse>()
        .await?
        .iroh;
    let servers = [server];

    let commands = collect_commands();
    let endpoint = Endpoint::builder()
        .secret_key(config.secret_key)
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
    let availables: Vec<String> = commands.keys().map(|name| name.to_string()).collect();
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
                availables.clone(),
            )
            .await?;
        clients.push(client);
        tracing::info!("joined {}", addr.node_id);
        let conn_type = endpoint
            .conn_type(addr.node_id)
            .ok()
            .and_then(|watcher| watcher.get().ok());
        tracing::info!("connection type {:?}", conn_type);
    }

    let client = command_factory::new_client(commands);
    client.bind_iroh(endpoint);

    tokio::signal::ctrl_c().await.ok();

    for client in clients {
        client.leave().await?;
        tracing::info!("left");
    }

    Ok(())
}
