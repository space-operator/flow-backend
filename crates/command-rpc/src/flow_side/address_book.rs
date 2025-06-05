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
use rand::{seq::SliceRandom, thread_rng};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::Mutex as AsyncMutex,
    task::{JoinHandle, spawn_local},
};
use url::Url;

pub use crate::command_capnp::address_book::*;
use crate::{
    command_side::command_factory::{self, CommandFactoryExt},
    r2p,
};

use super::remote_command::RemoteCommand;

pub const ALPN: &[u8] = b"space-operator/capnp-rpc/address-book/0";

#[derive(Clone)]
struct Info {
    direct_addresses: BTreeSet<SocketAddr>,
    relay_url: Url,
    availables: Vec<String>,
    client: Arc<AsyncMutex<Option<command_factory::Client>>>,
}

#[derive(Clone)]
pub struct AddressBook {
    factories: Arc<Mutex<BTreeMap<iroh::PublicKey, Info>>>,
    endpoint: Endpoint,
}

pub async fn connect_iroh(endpoint: Endpoint, addr: NodeAddr) -> Result<Client, anyhow::Error> {
    let connection = endpoint.connect(addr, ALPN).await.context("connect")?;
    let (writer, reader) = connection.open_bi().await.context("open_bi")?;
    Ok(connect_generic_futures_io(reader, writer))
}

impl AddressBook {
    pub fn bind_iroh(mut self, endpoint: Endpoint) -> JoinHandle<()> {
        endpoint.set_alpns([ALPN.to_vec()].into());
        self.endpoint = endpoint.clone();
        spawn_local(async move {
            while let Some(incoming) = endpoint.accept().await {
                if let Err(error) = spawn_rpc_system_handle(incoming, self.clone()).await {
                    tracing::error!("accept error: {}", error);
                }
            }
        })
    }

    pub async fn new_command(
        &self,
        name: &str,
        nd: &NodeData,
    ) -> Result<Box<dyn CommandTrait>, CommandError> {
        let factories_lock = self.factories.lock().unwrap();
        let factories = factories_lock
            .iter()
            .filter(|(_, v)| v.availables.iter().any(|c| c == name))
            .collect::<Vec<_>>();
        let (id, info) = factories
            .choose(&mut thread_rng())
            .ok_or_else(|| CommandError::msg("not found"))?;
        let node_id = (*id).clone();
        let info = (*info).clone();
        drop(factories_lock);

        let mut client = info.client.lock_owned().await;
        let client = if client.is_none() {
            let addr = NodeAddr {
                node_id,
                direct_addresses: info.direct_addresses,
                relay_url: Some(info.relay_url.into()),
            };
            let c = command_factory::connect_iroh(self.endpoint.clone(), addr).await?;
            *client = Some(c.clone());
            c
        } else {
            client.as_ref().unwrap().clone()
        };
        let cmd_client = client.init(name, nd).await?;
        let cmd = RemoteCommand::new(cmd_client).await?;

        Ok(Box::new(cmd))
    }
}

async fn spawn_rpc_system_handle(
    incoming: Incoming,
    book: AddressBook,
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
    book: AddressBook,
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
        self.book.factories.lock().unwrap().insert(
            self.remote_node_id,
            Info {
                direct_addresses,
                relay_url,
                availables,
                client: Arc::new(AsyncMutex::new(None)),
            },
        );
        Ok(())
    }

    fn leave_impl(&mut self) -> Result<(), capnp::Error> {
        self.book
            .factories
            .lock()
            .unwrap()
            .remove(&self.remote_node_id);
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
