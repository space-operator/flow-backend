use bincode::config::standard;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp::Side, twoparty::VatNetwork};
use futures::io::{BufReader, BufWriter};
use iroh::{Endpoint, endpoint::Incoming};
use serde::{Deserialize, Serialize};
use snafu::Whatever;
use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::task::{JoinHandle, spawn_local};
use url::Url;

pub use crate::command_capnp::address_book::*;
use crate::r2p;

#[derive(Serialize, Deserialize)]
struct Info {
    direct_addresses: BTreeSet<SocketAddr>,
    relay_url: Url,
}

#[derive(Clone)]
pub struct AddressBook {
    factories: Arc<Mutex<BTreeMap<iroh::PublicKey, Info>>>,
}

impl AddressBook {
    pub fn bind_iroh(self, endpoint: Endpoint) {
        spawn_local(async move {
            while let Some(incoming) = endpoint.accept().await {
                if let Err(error) = spawn_rpc_system_handle(incoming, self.clone()).await {
                    tracing::error!("accept error: {}", error);
                }
            }
        })
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
            bincode::decode_from_slice(params.get_direct_addresses()?, standard())?;
        let relay_url: Url = params.get_relay_url()?.to_str()?.parse()?;
        self.book.factories.lock().unwrap().insert(
            self.remote_node_id,
            Info {
                direct_addresses,
                relay_url,
            },
        );
        Ok(())
    }

    fn leave_impl(&mut self) -> capnp::capability::Promise<(), capnp::Error> {
        self.book
            .factories
            .lock()
            .unwrap()
            .remove(&self.remote_node_id);
        Ok(())
    }
}

impl Server for AddressBookConnection {
    fn join(
        &mut self,
        params: JoinParams,
        _: JoinResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        r2p(self
            .join_impl(params)
            .map_err(|error| capnp::Error::failed(error.to_string())))
    }

    fn leave(
        &mut self,
        _: LeaveParams,
        _: LeaveResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        r2p(self.leave_impl())
    }
}
