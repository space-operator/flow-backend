use crate::connect_generic_futures_io;
use anyhow::Context;
use bincode::config::standard;
use capnp::capability::Promise;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp::Side, twoparty::VatNetwork};
use flow_lib::{
    command::{CommandError, CommandTrait},
    config::client::NodeData,
};
use futures::io::{BufReader, BufWriter};
use iroh::{Endpoint, NodeAddr, endpoint::Incoming};
use parking_lot::RwLock;
use rand::{seq::SliceRandom, thread_rng};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    sync::Arc,
};
use tokio::task::{JoinHandle, spawn_local};
use url::Url;

pub use crate::command_capnp::address_book::*;
use crate::{
    command_side::command_factory::{self, CommandFactoryExt},
    r2p,
};

use super::remote_command::RemoteCommand;

pub const ALPN: &[u8] = b"space-operator/capnp-rpc/address-book/0";

pub struct ServerConfig {
    pub secret_key: iroh::SecretKey,
}

#[derive(Clone)]
struct Info {
    direct_addresses: BTreeSet<SocketAddr>,
    relay_url: Url,
    availables: Vec<String>,
}

#[derive(Clone)]
pub struct BaseAddressBook {
    factories: Arc<RwLock<BTreeMap<iroh::PublicKey, Info>>>,
    endpoint: Endpoint,
}

#[derive(Clone)]
pub struct AddressBook {
    base: BaseAddressBook,
    clients: BTreeMap<iroh::PublicKey, command_factory::Client>,
}

pub async fn connect_iroh(endpoint: Endpoint, addr: NodeAddr) -> Result<Client, anyhow::Error> {
    let connection = endpoint.connect(addr, ALPN).await.context("connect")?;
    let (writer, reader) = connection.open_bi().await.context("open_bi")?;
    Ok(connect_generic_futures_io(reader, writer))
}

impl BaseAddressBook {
    pub async fn new(config: ServerConfig) -> Result<Self, anyhow::Error> {
        let endpoint = Endpoint::builder()
            .secret_key(config.secret_key)
            .discovery_n0()
            .bind()
            .await
            .context("bind iroh")?;
        endpoint.set_alpns([ALPN.to_owned()].into());
        let this = Self {
            factories: Default::default(),
            endpoint: endpoint.clone(),
        };
        let book = this.clone();
        spawn_local(async move {
            while let Some(incoming) = endpoint.accept().await {
                if let Err(error) = spawn_rpc_system_handle(incoming, book.clone()).await {
                    tracing::error!("accept error: {}", error);
                }
            }
        });

        Ok(this)
    }
}

impl AddressBook {
    pub fn new(base: BaseAddressBook) -> Self {
        Self {
            base,
            clients: Default::default(),
        }
    }

    pub async fn new_command(
        &mut self,
        name: &str,
        nd: &NodeData,
    ) -> Result<Box<dyn CommandTrait>, CommandError> {
        let (node_id, info) = {
            let factories_lock = self.base.factories.read_arc();
            let factories = factories_lock
                .iter()
                .filter(|(_, v)| v.availables.iter().any(|c| c == name))
                .collect::<Vec<_>>();
            let (id, info) = factories
                .choose(&mut thread_rng())
                .ok_or_else(|| CommandError::msg("not found"))?;
            let node_id = (*id).clone();
            let info = (*info).clone();
            (node_id, info)
        };

        let cmd_client = match self.clients.entry(node_id) {
            std::collections::btree_map::Entry::Vacant(vacant_entry) => {
                let addr = NodeAddr {
                    node_id,
                    direct_addresses: info.direct_addresses,
                    relay_url: Some(info.relay_url.into()),
                };
                let client =
                    command_factory::connect_iroh(self.base.endpoint.clone(), addr).await?;
                let cmd_client = client.init(name, nd).await?;
                vacant_entry.insert(client);
                cmd_client
            }
            std::collections::btree_map::Entry::Occupied(occupied_entry) => {
                let client = occupied_entry.get();
                client.init(name, nd).await?
            }
        };

        let cmd = RemoteCommand::new(cmd_client).await?;

        Ok(Box::new(cmd))
    }
}

async fn spawn_rpc_system_handle(
    incoming: Incoming,
    book: BaseAddressBook,
) -> Result<JoinHandle<Result<(), capnp::Error>>, anyhow::Error> {
    let conn = incoming.await?;
    let remote_node_id = conn.remote_node_id()?;
    let client: Client = capnp_rpc::new_client(AddressBookConnection {
        book,
        remote_node_id,
    });
    let (send, recv) = conn.accept_bi().await?;
    let network = VatNetwork::new(
        BufReader::new(recv),
        BufWriter::new(send),
        Side::Server,
        <_>::default(),
    );
    let rpc_system = RpcSystem::new(Box::new(network), Some(client.clone().client));

    Ok(spawn_local(rpc_system))
}

struct AddressBookConnection {
    book: BaseAddressBook,
    remote_node_id: iroh::PublicKey,
}

impl AddressBookConnection {
    fn join_impl(&mut self, params: JoinParams) -> Result<(), anyhow::Error> {
        let params = params.get()?;
        let direct_addresses: BTreeSet<SocketAddr> =
            bincode::decode_from_slice(params.get_direct_addresses()?, standard())?.0;
        let relay_url: Url = params.get_relay_url()?.to_str()?.parse()?;
        let availables: Vec<String> =
            bincode::decode_from_slice(params.get_availables()?, standard())?.0;
        self.book.factories.write_arc().insert(
            self.remote_node_id,
            Info {
                direct_addresses,
                relay_url,
                availables,
            },
        );
        Ok(())
    }

    fn leave_impl(&mut self) -> Result<(), capnp::Error> {
        self.book.factories.write_arc().remove(&self.remote_node_id);
        Ok(())
    }
}

impl Server for AddressBookConnection {
    fn join(&mut self, params: JoinParams, _: JoinResults) -> Promise<(), capnp::Error> {
        r2p(self
            .join_impl(params)
            .map_err(|error| capnp::Error::failed(error.to_string())))
    }

    fn leave(&mut self, _: LeaveParams, _: LeaveResults) -> Promise<(), capnp::Error> {
        r2p(self.leave_impl())
    }
}

pub trait AddressBookExt {
    fn join(
        &self,
        direct_addresses: BTreeSet<SocketAddr>,
        relay_url: Url,
        availables: Vec<String>,
    ) -> impl Future<Output = Result<(), anyhow::Error>>;

    fn leave(&self) -> impl Future<Output = Result<(), anyhow::Error>>;
}

impl AddressBookExt for Client {
    async fn join(
        &self,
        direct_addresses: BTreeSet<SocketAddr>,
        relay_url: Url,
        availables: Vec<String>,
    ) -> Result<(), anyhow::Error> {
        let mut req = self.join_request();
        req.get().set_relay_url(relay_url.as_str());
        req.get()
            .set_availables(&bincode::encode_to_vec(&availables, standard())?);
        req.get()
            .set_direct_addresses(&bincode::encode_to_vec(&direct_addresses, standard())?);
        req.send().promise.await?;
        Ok(())
    }
    async fn leave(&self) -> Result<(), anyhow::Error> {
        self.leave_request().send().promise.await?;
        Ok(())
    }
}
