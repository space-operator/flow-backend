use crate::{anyhow2capnp, connect_generic_futures_io};
use anyhow::Context;
use bincode::config::standard;
use capnp::capability::Promise;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp::Side, twoparty::VatNetwork};
use flow_lib::{
    UserId,
    command::{CommandError, CommandIndex, CommandTrait, MatchCommand},
    config::client::NodeData,
};
use futures::io::{BufReader, BufWriter};
use iroh::{Endpoint, NodeAddr, endpoint::Incoming};
use rand::{seq::SliceRandom, thread_rng};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    rc::Rc,
    sync::{Arc, RwLock},
};
use tokio::{
    sync::Mutex as AsyncMutex,
    task::{JoinHandle, spawn_local},
};
use tower::{Service, ServiceExt};
use url::Url;

pub use crate::command_capnp::address_book::*;
use crate::command_side::command_factory::{self, CommandFactoryExt};

use super::remote_command::RemoteCommand;

pub const ALPN: &[u8] = b"space-operator/capnp-rpc/address-book/0";

/// For command factory authentication
pub mod authenticate {
    use flow_lib::{UserId, utils::TowerClient};
    use tower::service_fn;

    pub struct Request {
        pub pubkey: iroh::PublicKey,
        pub apikey: Option<String>,
    }

    #[derive(Clone, Debug)]
    pub enum Permission {
        All,
        User(UserId),
    }

    pub struct Response {
        pub permission: Permission,
    }

    pub type Error = anyhow::Error;

    pub type Svc = TowerClient<Request, Response, Error>;

    pub fn allow_all() -> Svc {
        TowerClient::new(service_fn(async |_| {
            Ok(Response {
                permission: Permission::All,
            })
        }))
    }
}

pub struct ServerConfig {
    pub secret_key: iroh::SecretKey,
}

#[derive(Clone)]
struct Info {
    direct_addresses: BTreeSet<SocketAddr>,
    relay_url: Url,
    availables: CommandIndex<()>,
    permission: authenticate::Permission,
}

#[derive(Clone)]
pub struct BaseAddressBook {
    factories: Arc<RwLock<BTreeMap<iroh::PublicKey, Info>>>,
    endpoint: Endpoint,
    auth: authenticate::Svc,
}

#[derive(Clone)]
pub struct AddressBook {
    base: BaseAddressBook,
    user_id: Option<UserId>,
    clients: Rc<AsyncMutex<BTreeMap<iroh::PublicKey, command_factory::Client>>>,
}

pub async fn connect_iroh(endpoint: Endpoint, addr: NodeAddr) -> Result<Client, anyhow::Error> {
    let connection = endpoint.connect(addr, ALPN).await.context("connect")?;
    let (writer, reader) = connection.open_bi().await.context("open_bi")?;
    Ok(connect_generic_futures_io(reader, writer))
}

impl BaseAddressBook {
    pub async fn new(config: ServerConfig, auth: authenticate::Svc) -> Result<Self, anyhow::Error> {
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
            auth,
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

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Returns a list of all connected providers with their info
    pub fn get_providers(
        &self,
    ) -> Vec<(
        iroh::PublicKey,
        BTreeSet<SocketAddr>,
        Url,
        Vec<MatchCommand>,
    )> {
        self.factories
            .read()
            .unwrap()
            .iter()
            .map(|(node_id, info)| {
                (
                    *node_id,
                    info.direct_addresses.clone(),
                    info.relay_url.clone(),
                    info.availables.availables().collect(),
                )
            })
            .collect()
    }

    /// Register the server itself as a provider with its native commands
    pub fn register_self(
        &self,
        direct_addresses: BTreeSet<SocketAddr>,
        relay_url: Url,
        availables: Vec<MatchCommand>,
    ) {
        let node_id = self.endpoint.node_id();
        tracing::info!(
            "Registering server {} with {} commands",
            node_id,
            availables.len()
        );
        
        self.factories.write().unwrap().insert(
            node_id,
            Info {
                direct_addresses,
                relay_url,
                availables: availables.into_iter().map(|m| (m, ())).collect(),
                permission: authenticate::Permission::All,
            },
        );
    }
}

impl AddressBook {
    pub fn new(base: BaseAddressBook, user_id: Option<UserId>) -> Self {
        Self {
            base,
            user_id,
            clients: Default::default(),
        }
    }

    async fn get_or_connect(
        &self,
        node_id: iroh::PublicKey,
    ) -> Result<command_factory::Client, CommandError> {
        let mut clients = self.clients.lock().await;
        let client = match clients.get(&node_id) {
            Some(client) => client.clone(),
            None => {
                let info = self
                    .base
                    .factories
                    .read()
                    .unwrap()
                    .get(&node_id)
                    .unwrap()
                    .clone();
                let addr = NodeAddr {
                    node_id,
                    direct_addresses: info.direct_addresses,
                    relay_url: Some(info.relay_url.into()),
                };
                let client =
                    command_factory::connect_iroh(self.base.endpoint.clone(), addr).await?;
                clients.insert(node_id, client.clone());
                client
            }
        };
        Ok(client)
    }

    pub async fn init(
        &mut self,
        nd: &NodeData,
    ) -> Result<Option<Box<dyn CommandTrait>>, CommandError> {
        let node_id = {
            let factories_lock = self.base.factories.read().unwrap();
            let factories = factories_lock
                .iter()
                .filter(|(_, v)| {
                    let can_use = match v.permission {
                        authenticate::Permission::All => true,
                        authenticate::Permission::User(id) => Some(id) == self.user_id,
                    };
                    let contain = v.availables.get(nd.r#type, &nd.node_id).is_some();
                    can_use && contain
                })
                .map(|(k, _)| k)
                .collect::<Vec<_>>();
            **factories
                .choose(&mut thread_rng())
                .ok_or_else(|| CommandError::msg("not found"))?
        };

        let factory_client = self.get_or_connect(node_id).await?;
        let cmd_client = factory_client.init(nd).await?;

        match cmd_client {
            Some(client) => Ok(Some(Box::new(RemoteCommand::new(client).await?))),
            None => Ok(None),
        }
    }

    pub fn availables(&self) -> impl Iterator<Item = MatchCommand> {
        let factories = self.base.factories.read().unwrap();
        factories
            .values()
            .flat_map(|i| i.availables.availables())
            .collect::<Vec<_>>()
            .into_iter()
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

#[derive(Clone)]
struct AddressBookConnection {
    book: BaseAddressBook,
    remote_node_id: iroh::PublicKey,
}

impl AddressBookConnection {
    async fn join_impl(&mut self, params: JoinParams) -> Result<(), anyhow::Error> {
        let params = params.get()?;
        let direct_addresses: BTreeSet<SocketAddr> =
            bincode::decode_from_slice(params.get_direct_addresses()?, standard())?.0;
        let relay_url: Url = params.get_relay_url()?.to_str()?.parse()?;
        let availables: Vec<MatchCommand> =
            bincode::decode_from_slice(params.get_availables()?, standard())?.0;
        let apikey = params
            .get_apikey()
            .ok()
            .and_then(|key| key.to_str().ok())
            .map(|key| key.to_owned());
        let permission = self
            .book
            .auth
            .ready()
            .await?
            .call(authenticate::Request {
                pubkey: self.remote_node_id,
                apikey,
            })
            .await?
            .permission;
        tracing::info!(
            "node {} joined, permission: {:?}, availables: {:?}",
            self.remote_node_id,
            permission,
            availables
        );
        self.book.factories.write().unwrap().insert(
            self.remote_node_id,
            Info {
                direct_addresses,
                relay_url,
                availables: availables.into_iter().map(|m| (m, ())).collect(),
                permission,
            },
        );
        Ok(())
    }

    fn leave_impl(&mut self) -> Result<(), capnp::Error> {
        if self
            .book
            .factories
            .write()
            .unwrap()
            .remove(&self.remote_node_id)
            .is_some()
        {
            tracing::info!("node {} left", self.remote_node_id);
        }
        Ok(())
    }
}

impl Server for AddressBookConnection {
    fn join(&mut self, params: JoinParams, _: JoinResults) -> Promise<(), capnp::Error> {
        let mut this = self.clone();
        Promise::from_future(async move { this.join_impl(params).await.map_err(anyhow2capnp) })
    }

    fn leave(&mut self, _: LeaveParams, _: LeaveResults) -> Promise<(), capnp::Error> {
        self.leave_impl().into()
    }
}

pub trait AddressBookExt {
    fn join(
        &self,
        direct_addresses: BTreeSet<SocketAddr>,
        relay_url: Url,
        availables: &[MatchCommand],
        apikey: Option<String>,
    ) -> impl Future<Output = Result<(), anyhow::Error>>;

    fn leave(&self) -> impl Future<Output = Result<(), anyhow::Error>>;
}

impl AddressBookExt for Client {
    async fn join(
        &self,
        direct_addresses: BTreeSet<SocketAddr>,
        relay_url: Url,
        availables: &[MatchCommand],
        apikey: Option<String>,
    ) -> Result<(), anyhow::Error> {
        let mut req = self.join_request();
        req.get().set_relay_url(relay_url.as_str());
        req.get()
            .set_availables(&bincode::encode_to_vec(availables, standard())?);
        req.get()
            .set_direct_addresses(&bincode::encode_to_vec(&direct_addresses, standard())?);
        if let Some(key) = apikey {
            req.get().set_apikey(&key);
        }
        req.send().promise.await?;
        Ok(())
    }
    async fn leave(&self) -> Result<(), anyhow::Error> {
        self.leave_request().send().promise.await?;
        Ok(())
    }
}
