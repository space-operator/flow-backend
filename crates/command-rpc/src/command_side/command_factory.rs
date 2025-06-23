use crate::connect_generic_futures_io;
use bincode::config::standard;
use capnp::capability::Promise;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp::Side, twoparty::VatNetwork};
use flow_lib::{
    command::{CommandDescription, CommandError},
    config::client::NodeData,
};
use futures::io::{BufReader, BufWriter};
use iroh::{Endpoint, NodeAddr, endpoint::Incoming};
use iroh_quinn::ConnectionError;
use snafu::prelude::*;
use std::{borrow::Cow, collections::BTreeMap, str::Utf8Error};
use tokio::task::{JoinHandle, spawn_local};
use tracing::{Instrument, Level, Span, span};

pub use crate::command_capnp::command_factory::*;
use crate::command_side::command_trait;

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum AllAvailablesError {
    Capnp {
        source: capnp::Error,
        context: String,
    },
    BincodeDecode {
        source: bincode::error::DecodeError,
    },
    BincodeEncode {
        source: bincode::error::EncodeError,
    },
}

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum ConnectError {
    Connect { source: anyhow::Error },
    OpenBi { source: ConnectionError },
}

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum InitError {
    Capnp {
        source: capnp::Error,
        context: String,
    },
    Utf8 {
        source: Utf8Error,
    },
    Json {
        source: serde_json::Error,
        context: String,
    },
    SimdJson {
        source: simd_json::Error,
        context: String,
    },
    NewCommand {
        source: CommandError,
    },
    NotAvailable {
        name: String,
    },
}

pub const ALPN: &[u8] = b"space-operator/capnp-rpc/command-factory/0";

pub fn new_client(availables: BTreeMap<Cow<'static, str>, &'static CommandDescription>) -> Client {
    capnp_rpc::new_client(CommandFactoryImpl { availables })
}

pub async fn connect_iroh(endpoint: Endpoint, addr: NodeAddr) -> Result<Client, ConnectError> {
    async move {
        use connect_error::*;
        let connection = endpoint.connect(addr, ALPN).await.context(ConnectSnafu)?;
        let (writer, reader) = connection.open_bi().await.context(OpenBiSnafu)?;
        Ok(connect_generic_futures_io(reader, writer))
    }
    .instrument(span!(parent: None, Level::INFO, "iroh_connection"))
    .await
}

pub trait CommandFactoryExt {
    fn init(
        &self,
        name: &str,
        nd: &NodeData,
    ) -> impl Future<Output = Result<command_trait::Client, InitError>>;
    fn all_availables(&self) -> impl Future<Output = Result<Vec<String>, AllAvailablesError>>;
    fn bind_iroh(&self, endpoint: Endpoint) -> JoinHandle<()>;
}

impl CommandFactoryExt for Client {
    async fn init(&self, name: &str, nd: &NodeData) -> Result<command_trait::Client, InitError> {
        use init_error::*;
        let mut req = self.init_request();
        req.get().set_name(name);
        req.get()
            .set_nd(&simd_json::to_vec(nd).context(SimdJsonSnafu {
                context: "serialize NodeData",
            })?);
        let client = req
            .send()
            .promise
            .await
            .context(CapnpSnafu { context: "send" })?
            .get()
            .context(CapnpSnafu { context: "get" })?
            .get_cmd()
            .context(CapnpSnafu { context: "get_cmd" })?;
        Ok(client)
    }

    async fn all_availables(&self) -> Result<Vec<String>, AllAvailablesError> {
        use all_availables_error::*;
        let resp = self
            .all_availables_request()
            .send()
            .promise
            .await
            .context(CapnpSnafu { context: "send" })?;
        let data = resp
            .get()
            .context(CapnpSnafu { context: "get" })?
            .get_availables()
            .context(CapnpSnafu {
                context: "get_availables",
            })?;
        let names = bincode::decode_from_slice(data, standard())
            .context(BincodeDecodeSnafu)?
            .0;
        Ok(names)
    }

    fn bind_iroh(&self, endpoint: Endpoint) -> JoinHandle<()> {
        let client = self.clone();
        endpoint.set_alpns([ALPN.to_vec()].into());
        spawn_local(async move {
            while let Some(incoming) = endpoint.accept().await {
                if let Err(error) = spawn_rpc_system_handle(incoming, client.clone()).await {
                    tracing::error!("accept error: {}", error);
                }
            }
        })
    }
}

async fn spawn_rpc_system_handle(
    incoming: Incoming,
    factory: Client,
) -> Result<JoinHandle<Result<(), capnp::Error>>, ConnectionError> {
    let connection = incoming.await?;
    let (sink, stream) = connection.accept_bi().await?;
    let network = VatNetwork::new(
        BufReader::new(stream),
        BufWriter::new(sink),
        Side::Server,
        <_>::default(),
    );
    let rpc_system = RpcSystem::new(Box::new(network), Some(factory.clone().client));
    Ok(spawn_local(rpc_system))
}

pub struct CommandFactoryImpl {
    availables: BTreeMap<Cow<'static, str>, &'static CommandDescription>,
}

impl CommandFactoryImpl {
    fn init_impl(&mut self, params: InitParams, mut results: InitResults) -> Result<(), InitError> {
        use init_error::*;
        let params = params
            .get()
            .context(init_error::CapnpSnafu { context: "get" })?;

        let name = params
            .get_name()
            .context(init_error::CapnpSnafu {
                context: "get_name",
            })?
            .to_str()
            .context(init_error::Utf8Snafu)?;
        if let Some(description) = self.availables.get(name) {
            let nd = params
                .get_nd()
                .context(init_error::CapnpSnafu { context: "get_nd" })?;
            let nd: NodeData = serde_json::from_slice(nd).context(JsonSnafu {
                context: "parse NodeData",
            })?;
            tracing::info!("init {}", name);
            let cmd = (description.fn_new)(&nd).context(NewCommandSnafu)?;
            results.get().set_cmd(command_trait::new_client(cmd));
            Ok(())
        } else {
            init_error::NotAvailableSnafu {
                name: name.to_owned(),
            }
            .fail()
        }
    }

    fn all_availables_impl(
        &self,
        mut results: AllAvailablesResults,
    ) -> Result<(), AllAvailablesError> {
        let names = self.availables.keys().collect::<Vec<_>>();
        let names = bincode::encode_to_vec(&names, standard())
            .context(all_availables_error::BincodeEncodeSnafu)?;
        results.get().set_availables(&names);
        Ok(())
    }
}

impl Server for CommandFactoryImpl {
    fn init(&mut self, params: InitParams, results: InitResults) -> Promise<(), capnp::Error> {
        match self.init_impl(params, results) {
            Ok(_) => Promise::ok(()),
            Err(error) => Promise::err(capnp::Error::failed(error.to_string())),
        }
    }

    fn all_availables(
        &mut self,
        _: AllAvailablesParams,
        results: AllAvailablesResults,
    ) -> Promise<(), capnp::Error> {
        match self.all_availables_impl(results) {
            Ok(_) => Promise::ok(()),
            Err(error) => Promise::err(capnp::Error::failed(error.to_string())),
        }
    }
}
