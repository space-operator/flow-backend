use crate::command_capnp::{command_factory, command_trait, command_trait::run_params};
use bincode::config::standard;
use capnp::capability::Promise;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp::Side, twoparty::VatNetwork};
use flow_lib::{
    Value,
    command::{CommandDescription, CommandError, CommandTrait},
    config::client::NodeData,
    context::{CommandContext, CommandContextData, FlowServices, FlowSetServices},
    utils::tower_client::unimplemented_svc,
    value::{
        self,
        bincode_impl::{map_from_bincode, map_to_bincode},
    },
};
use futures::{
    AsyncReadExt, TryFutureExt,
    future::LocalBoxFuture,
    io::{BufReader, BufWriter},
};
use iroh::{Endpoint, NodeAddr, RelayUrl, SecretKey, endpoint::Incoming};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    str::Utf8Error,
    sync::Arc,
};
use thiserror::Error as ThisError;
use tokio::{
    net::ToSocketAddrs,
    sync::{Mutex, oneshot},
    task::{JoinHandle, spawn_local},
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[derive(ThisError, Debug)]
pub enum Error {
    #[error(transparent)]
    Any(#[from] anyhow::Error),
    #[error(transparent)]
    IrohWatch(#[from] iroh::watchable::Disconnected),
    #[error(transparent)]
    IrohConnection(#[from] iroh::endpoint::ConnectionError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Value(#[from] value::Error),
    #[error(transparent)]
    BincodeDecode(#[from] bincode::error::DecodeError),
    #[error(transparent)]
    BincodeEncode(#[from] bincode::error::EncodeError),
    #[error("data contain invalid UTF-8")]
    Utf8(#[from] Utf8Error),
    #[error("command is not available: {:?}", .0)]
    NotAvailable(String),
    #[error(transparent)]
    Cap(#[from] capnp::Error),
    #[error(transparent)]
    NewCommand(CommandError),
    #[error(transparent)]
    Run(CommandError),
    #[error(transparent)]
    SimdJson(#[from] simd_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub trait CommandFactoryExt {
    fn iroh_address(&self) -> impl Future<Output = Result<IrohAddress, capnp::Error>>;
}

impl CommandFactoryExt for command_factory::Client {
    async fn iroh_address(&self) -> Result<IrohAddress, capnp::Error> {
        let resp = self.iroh_address_request().send().promise.await?;
        let data = resp.get()?.get_address()?;
        let addr = bincode::serde::decode_from_slice(data, standard())
            .map_err(Error::from)?
            .0;
        Ok(addr)
    }
}

impl From<Error> for capnp::Error {
    fn from(value: Error) -> Self {
        match value {
            Error::Cap(error) => error,
            error => capnp::Error::failed(error.to_string()),
        }
    }
}

pub struct CommandFactoryImpl {
    availables: BTreeMap<Cow<'static, str>, &'static CommandDescription>,
    iroh_endpoint: Option<Endpoint>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct IrohAddress {
    pub node_id: iroh::NodeId,
    pub direct_addresses: BTreeSet<SocketAddr>,
    pub relay_url: RelayUrl,
}

impl CommandFactoryImpl {
    fn init_impl(
        &mut self,
        params: command_factory::InitParams,
        mut results: command_factory::InitResults,
    ) -> Result<(), Error> {
        let name = params.get()?.get_name()?.to_str()?;
        if let Some(description) = self.availables.get(name) {
            let nd = params.get()?.get_nd()?;
            let nd: NodeData = serde_json::from_slice(nd)?;
            let cmd = (description.fn_new)(&nd).map_err(Error::NewCommand)?;
            let cmd = Arc::new(Mutex::new(cmd));
            results
                .get()
                .set_cmd(capnp_rpc::new_client(CommandTraitImpl { cmd }));
            Ok(())
        } else {
            Err(Error::NotAvailable(name.to_owned()))
        }
    }

    fn all_availables_impl(
        &self,
        mut results: command_factory::AllAvailablesResults,
    ) -> Result<(), Error> {
        let names = self.availables.keys().collect::<Vec<_>>();
        let names = bincode::encode_to_vec(&names, standard())?;
        results.get().set_availables(&names);
        Ok(())
    }

    fn iroh_address_impl(
        &self,
        mut results: command_factory::IrohAddressResults,
    ) -> LocalBoxFuture<'static, Result<(), capnp::Error>> {
        let e = self.iroh_endpoint.clone();
        Box::pin(async move {
            if let Some(e) = e {
                let node_id = e.node_id();
                let direct_addresses = e
                    .direct_addresses()
                    .initialized()
                    .await
                    .map_err(Error::from)?
                    .into_iter()
                    .map(|addr| addr.addr)
                    .collect::<BTreeSet<_>>();
                let relay_url = e.home_relay().initialized().await.map_err(Error::from)?;
                results.get().set_address(
                    &bincode::serde::encode_to_vec(
                        &IrohAddress {
                            node_id,
                            direct_addresses,
                            relay_url,
                        },
                        standard(),
                    )
                    .map_err(Error::from)?,
                );
            }
            Ok(())
        })
    }

    pub async fn serve_iroh(
        mut self,
        key: SecretKey,
    ) -> Result<(command_factory::Client, JoinHandle<()>), Error> {
        if self.iroh_endpoint.is_none() {
            self.iroh_endpoint = Some(
                Endpoint::builder()
                    .secret_key(key)
                    .alpns([b"space-operator/capnp-rpc/command-factory/0".to_vec()].into())
                    .bind()
                    .await?,
            );
        }
        let endpoint = self.iroh_endpoint.clone().unwrap();
        let client: command_factory::Client = capnp_rpc::new_client(self);

        let handle = spawn_local({
            let client = client.clone();
            async move {
                while let Some(incoming) = endpoint.accept().await {
                    if let Err(error) = spawn_rpc_system_handle(incoming, client.clone()).await {
                        tracing::error!("accept error: {}", error);
                    }
                }
            }
        });

        Ok((client, handle))
    }
}

pub async fn connect_iroh_command_factory(
    addr: IrohAddress,
    key: SecretKey,
) -> Result<command_factory::Client, Error> {
    let endpoint = Endpoint::builder()
        .secret_key(key)
        .alpns([b"space-operator/capnp-rpc/command-factory/0".to_vec()].into())
        .discovery_n0()
        .bind()
        .await?;
    let connection = endpoint
        .connect(
            NodeAddr {
                node_id: addr.node_id,
                direct_addresses: addr.direct_addresses.into_iter().collect(),
                relay_url: Some("https://aps1-1.relay.iroh.network./".parse().unwrap()),
            },
            b"space-operator/capnp-rpc/command-factory/0",
        )
        .await?;
    let (writer, reader) = connection.open_bi().await?;
    connect_generic_futures_io(reader, writer)
}

async fn spawn_rpc_system_handle(
    incoming: Incoming,
    factory: command_factory::Client,
) -> Result<JoinHandle<Result<(), capnp::Error>>, Error> {
    let connection = incoming.await?;
    let (sink, stream) = connection.accept_bi().await?;
    let network = VatNetwork::new(
        BufReader::new(stream.compat()),
        BufWriter::new(sink.compat_write()),
        Side::Server,
        <_>::default(),
    );
    let rpc_system = RpcSystem::new(Box::new(network), Some(factory.clone().client));
    Ok(spawn_local(rpc_system))
}

impl command_factory::Server for CommandFactoryImpl {
    fn init(
        &mut self,
        params: command_factory::InitParams,
        results: command_factory::InitResults,
    ) -> Promise<(), capnp::Error> {
        match self.init_impl(params, results) {
            Ok(_) => Promise::ok(()),
            Err(error) => Promise::err(error.into()),
        }
    }

    fn all_availables(
        &mut self,
        _: command_factory::AllAvailablesParams,
        results: command_factory::AllAvailablesResults,
    ) -> ::capnp::capability::Promise<(), ::capnp::Error> {
        match self.all_availables_impl(results) {
            Ok(_) => Promise::ok(()),
            Err(error) => Promise::err(error.into()),
        }
    }

    fn iroh_address(
        &mut self,
        _: command_factory::IrohAddressParams,
        results: command_factory::IrohAddressResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        Promise::from_future(self.iroh_address_impl(results))
    }
}

pub struct CommandTraitImpl {
    cmd: Arc<Mutex<Box<dyn CommandTrait>>>,
}

fn parse_inputs(params: run_params::Reader<'_>) -> Result<value::Map, Error> {
    let inputs = params.get_inputs()?;
    Ok(map_from_bincode(inputs)?)
}

impl CommandTraitImpl {
    fn run_impl(
        &mut self,
        params: command_trait::RunParams,
        mut results: command_trait::RunResults,
    ) -> impl Future<Output = Result<(), Error>> + 'static {
        let cmd = self.cmd.clone();
        async move {
            let inputs = parse_inputs(params.get()?)?;
            let context = params.get()?.get_ctx()?;
            let data: CommandContextData = value::from_value(Value::from_bincode(
                context
                    .data_request()
                    .send()
                    .promise
                    .await?
                    .get()?
                    .get_data()?,
            )?)?;
            let result = cmd
                .lock_owned()
                .await
                .run(
                    CommandContext::builder()
                        .execute(unimplemented_svc())
                        .get_jwt(unimplemented_svc())
                        .flow(FlowServices {
                            signer: unimplemented_svc(),
                            set: FlowSetServices {
                                http: reqwest::Client::new(),
                                solana_client: Arc::new(data.flow.set.solana.build_client()),
                                extensions: Arc::new(Default::default()),
                                api_input: unimplemented_svc(),
                            },
                        })
                        .data(data)
                        .build(),
                    inputs,
                )
                .await
                .map_err(Error::Run)?;
            results.get().set_output(&map_to_bincode(&result)?);
            Ok(())
        }
    }
}

impl command_trait::Server for CommandTraitImpl {
    fn run(
        &mut self,
        params: command_trait::RunParams,
        results: command_trait::RunResults,
    ) -> Promise<(), capnp::Error> {
        Promise::from_future(self.run_impl(params, results).map_err(Into::into))
    }

    fn name(
        &mut self,
        _: command_trait::NameParams,
        mut results: command_trait::NameResults,
    ) -> Promise<(), capnp::Error> {
        let cmd = self.cmd.clone();
        Promise::from_future(async move {
            let name = cmd.lock().await.name();
            results.get().set_name(name);
            Ok(())
        })
    }

    fn inputs(
        &mut self,
        _: command_trait::InputsParams,
        mut results: command_trait::InputsResults,
    ) -> Promise<(), capnp::Error> {
        let cmd = self.cmd.clone();
        Promise::from_future(
            async move {
                let inputs = cmd.lock().await.inputs();
                let inputs = simd_json::to_vec(&inputs)?;
                results.get().set_inputs(&inputs);
                Ok::<_, Error>(())
            }
            .map_err(Into::into),
        )
    }

    fn outputs(
        &mut self,
        _: command_trait::OutputsParams,
        mut results: command_trait::OutputsResults,
    ) -> Promise<(), capnp::Error> {
        let cmd = self.cmd.clone();
        Promise::from_future(
            async move {
                let outputs = cmd.lock().await.outputs();
                let outputs = simd_json::to_vec(&outputs)?;
                results.get().set_outputs(&outputs);
                Ok::<_, Error>(())
            }
            .map_err(Into::into),
        )
    }

    fn instruction_info(
        &mut self,
        _: command_trait::InstructionInfoParams,
        mut results: command_trait::InstructionInfoResults,
    ) -> Promise<(), capnp::Error> {
        let cmd = self.cmd.clone();
        Promise::from_future(
            async move {
                let info = cmd.lock().await.instruction_info();
                let info = simd_json::to_vec(&info)?;
                results.get().set_info(&info);
                Ok::<_, Error>(())
            }
            .map_err(Into::into),
        )
    }

    fn permissions(
        &mut self,
        _: command_trait::PermissionsParams,
        mut results: command_trait::PermissionsResults,
    ) -> Promise<(), capnp::Error> {
        let cmd = self.cmd.clone();
        Promise::from_future(
            async move {
                let perm = cmd.lock().await.permissions();
                let perm = simd_json::to_vec(&perm)?;
                results.get().set_permissions(&perm);
                Ok::<_, Error>(())
            }
            .map_err(Into::into),
        )
    }
}

pub async fn serve(
    addr: impl ToSocketAddrs,
    factory: command_factory::Client,
    local_addr: Option<oneshot::Sender<SocketAddr>>,
) -> Result<(), std::io::Error> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    if let Some(tx) = local_addr {
        let local = listener.local_addr()?;
        tx.send(local).ok();
    }
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            tracing::error!("error accepting connection");
            continue;
        };
        let (read, write) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
        let network = VatNetwork::new(
            BufReader::new(read),
            BufWriter::new(write),
            Side::Server,
            <_>::default(),
        );
        let rpc_system = RpcSystem::new(Box::new(network), Some(factory.clone().client));
        spawn_local(rpc_system);
    }
}

fn connect_generic<
    R: tokio::io::AsyncRead + Unpin + 'static,
    W: tokio::io::AsyncWrite + Unpin + 'static,
>(
    reader: R,
    writer: W,
) -> Result<command_factory::Client, Error> {
    connect_generic_futures_io(reader.compat(), writer.compat_write())
}

fn connect_generic_futures_io<
    R: futures::io::AsyncRead + Unpin + 'static,
    W: futures::io::AsyncWrite + Unpin + 'static,
>(
    reader: R,
    writer: W,
) -> Result<command_factory::Client, Error> {
    let network = Box::new(VatNetwork::new(
        futures::io::BufReader::new(reader),
        futures::io::BufWriter::new(writer),
        Side::Client,
        Default::default(),
    ));
    let mut rpc_system = RpcSystem::new(network, None);
    let client: command_factory::Client = rpc_system.bootstrap(Side::Server);
    tokio::task::spawn_local(rpc_system);
    Ok(client)
}

pub async fn connect_command_factory(
    addr: impl ToSocketAddrs,
) -> Result<command_factory::Client, Error> {
    let stream = tokio::net::TcpStream::connect(addr).await?;
    let (reader, writer) = stream.into_split();
    connect_generic(reader, writer)
}

#[cfg(test)]
mod tests;
