use std::future::ready;

use crate::{anyhow2capnp, connect_generic_futures_io, tracing::TrackFlowRun};
use anyhow::{Context, anyhow};
use bincode::config::standard;
use capnp::{ErrorKind, capability::Promise};
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp::Side, twoparty::VatNetwork};
use flow_lib::{command::CommandFactory, config::client::NodeData};
use futures::{
    TryFutureExt,
    future::LocalBoxFuture,
    io::{BufReader, BufWriter},
};
use iroh::{Endpoint, NodeAddr, endpoint::Incoming};
use iroh_quinn::ConnectionError;
use tokio::task::{JoinHandle, spawn_local};
use tracing::{Instrument, Level, span};

pub use crate::command_capnp::command_factory::*;
use crate::command_side::command_trait;

pub const ALPN: &[u8] = b"space-operator/capnp-rpc/command-factory/0";

pub fn new_client(factory: CommandFactory, tracker: TrackFlowRun) -> Client {
    capnp_rpc::new_client(CommandFactoryImpl { factory, tracker })
}

pub async fn connect_iroh(endpoint: Endpoint, addr: NodeAddr) -> Result<Client, anyhow::Error> {
    async move {
        let connection = endpoint.connect(addr, ALPN).await.context("iroh connect")?;
        let (writer, reader) = connection.open_bi().await.context("iron open_bi")?;
        Ok(connect_generic_futures_io(reader, writer))
    }
    .instrument(span!(parent: None, Level::INFO, "iroh_connection"))
    .await
}

pub trait CommandFactoryExt {
    fn init(
        &self,
        nd: &NodeData,
    ) -> impl Future<Output = Result<Option<command_trait::Client>, anyhow::Error>>;
    fn all_availables(&self) -> impl Future<Output = Result<Vec<String>, anyhow::Error>>;
    fn bind_iroh(&self, endpoint: Endpoint) -> JoinHandle<()>;
}

impl CommandFactoryExt for Client {
    async fn init(&self, nd: &NodeData) -> Result<Option<command_trait::Client>, anyhow::Error> {
        let mut req = self.init_request();
        req.get()
            .set_nd(&simd_json::to_vec(nd).context("simd_json serialize NodeData")?);
        let result = req
            .send()
            .promise
            .await
            .context("send init_request")?
            .get()
            .context("get")?
            .get_cmd();
        match result {
            Ok(cmd) => Ok(Some(cmd)),
            Err(error) => {
                if error.kind == ErrorKind::FieldNotFound {
                    Ok(None)
                } else {
                    Err(anyhow!(error).context("get_cmd"))
                }
            }
        }
    }

    async fn all_availables(&self) -> Result<Vec<String>, anyhow::Error> {
        let resp = self
            .all_availables_request()
            .send()
            .promise
            .await
            .context("send all_availables_request")?;
        let data = resp
            .get()
            .context("get")?
            .get_availables()
            .context("get_availables")?;
        let names = bincode::decode_from_slice(data, standard())
            .context("bincode decode availables")?
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
    factory: CommandFactory,
    tracker: TrackFlowRun,
}

impl CommandFactoryImpl {
    fn init_impl(
        &mut self,
        params: InitParams,
        mut results: InitResults,
    ) -> LocalBoxFuture<'static, Result<(), anyhow::Error>> {
        let nd = (move || {
            let nd = params.get().context("get")?.get_nd().context("get_nd")?;
            let nd: NodeData =
                serde_json::from_slice(nd).context("serde_json deserialize NodeData")?;
            Ok(nd)
        })();

        match nd {
            Ok(nd) => {
                let fut = self.factory.init(&nd);
                let tracker = self.tracker.clone();
                Box::pin(async move {
                    if let Some(cmd) = fut.await? {
                        results
                            .get()
                            .set_cmd(command_trait::new_client(cmd, tracker));
                    }
                    Ok(())
                })
            }
            Err(error) => return Box::pin(ready(Err(error))),
        }
    }

    fn all_availables_impl(&self, mut results: AllAvailablesResults) -> Result<(), anyhow::Error> {
        let vec = self.factory.availables().collect::<Vec<_>>();
        let data =
            bincode::encode_to_vec(&vec, standard()).context("bincode::Encode availables")?;
        results.get().set_availables(&data);
        Ok(())
    }
}

impl Server for CommandFactoryImpl {
    fn init(&mut self, params: InitParams, results: InitResults) -> Promise<(), capnp::Error> {
        Promise::from_future(self.init_impl(params, results).map_err(anyhow2capnp))
    }

    fn all_availables(
        &mut self,
        _: AllAvailablesParams,
        results: AllAvailablesResults,
    ) -> Promise<(), capnp::Error> {
        match self.all_availables_impl(results) {
            Ok(_) => Promise::ok(()),
            Err(error) => Promise::err(anyhow2capnp(error)),
        }
    }
}
