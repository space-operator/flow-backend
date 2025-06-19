use capnp::capability::Promise;
use flow_lib::{
    Value,
    command::{CommandError, CommandTrait},
    config::SolanaReqwestClient,
    context::{CommandContext, CommandContextData, FlowServices, FlowSetServices},
    utils::tower_client::unimplemented_svc,
    value::{
        self,
        bincode_impl::{map_from_bincode, map_to_bincode},
    },
};
use futures::TryFutureExt;
use snafu::{ResultExt, Snafu};
use std::{
    rc::Rc,
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

pub use crate::command_capnp::command_trait::*;

#[derive(Debug, Snafu)]
pub enum Error {
    Capnp {
        source: capnp::Error,
        context: String,
    },
    BincodeDecode {
        source: bincode::error::DecodeError,
        context: String,
    },
    BincodeEncode {
        source: bincode::error::EncodeError,
        context: String,
    },
    Value {
        source: value::Error,
        context: String,
    },
    Run {
        source: CommandError,
    },
    SimdJson {
        source: simd_json::Error,
        context: String,
    },
}

pub fn new_client(cmd: Box<dyn CommandTrait>) -> Client {
    capnp_rpc::new_client(CommandTraitImpl {
        cmd: Rc::new(Mutex::new(cmd)),
    })
}

struct CommandTraitImpl {
    cmd: Rc<Mutex<Box<dyn CommandTrait>>>,
}

impl From<Error> for capnp::Error {
    fn from(value: Error) -> Self {
        capnp::Error::failed(value.to_string())
    }
}

fn parse_inputs(params: run_params::Reader<'_>) -> Result<value::Map, Error> {
    let inputs = params.get_inputs().context(CapnpSnafu {
        context: "get_inputs",
    })?;
    Ok(map_from_bincode(inputs).context(BincodeDecodeSnafu {
        context: "decode map",
    })?)
}

// TODO: old flow-lib code use reqwest client with 30 secs timeout
pub(crate) static SOLANA_HTTP_CLIENT: LazyLock<SolanaReqwestClient> =
    LazyLock::new(|| Default::default());
pub(crate) static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| Default::default());

impl CommandTraitImpl {
    fn run_impl(
        &mut self,
        params: RunParams,
        mut results: RunResults,
    ) -> impl Future<Output = Result<(), Error>> + 'static {
        let cmd = self.cmd.clone();
        async move {
            let now = Instant::now();
            let params = params.get().context(CapnpSnafu { context: "get" })?;
            let inputs = parse_inputs(params)?;
            let context = params
                .get_ctx()
                .context(CapnpSnafu { context: "get_ctx" })?;
            let resp = context
                .data_request()
                .send()
                .promise
                .await
                .context(CapnpSnafu { context: "send" })?;
            let data = resp
                .get()
                .context(CapnpSnafu { context: "get" })?
                .get_data()
                .context(CapnpSnafu {
                    context: "get_data",
                })?;
            let value = Value::from_bincode(data).context(BincodeDecodeSnafu {
                context: "decode value",
            })?;
            let data: CommandContextData = value::from_value(value).context(ValueSnafu {
                context: "decode CommandContextData",
            })?;
            let ctx = CommandContext::builder()
                .execute(unimplemented_svc())
                .get_jwt(unimplemented_svc())
                .flow(FlowServices {
                    signer: unimplemented_svc(),
                    set: FlowSetServices {
                        http: HTTP_CLIENT.clone(),
                        solana_client: Arc::new(
                            data.flow
                                .set
                                .solana
                                .build_client(Some(SOLANA_HTTP_CLIENT.clone())),
                        ),
                        extensions: Arc::new(Default::default()),
                        api_input: unimplemented_svc(),
                    },
                })
                .data(data)
                .build();
            let id = *ctx.node_id();
            let times = *ctx.times();
            let result = cmd.lock().await.run(ctx, inputs).await.context(RunSnafu)?;
            results
                .get()
                .set_output(&map_to_bincode(&result).context(BincodeEncodeSnafu {
                    context: "encode map",
                })?);
            tracing::info!("ran {}:{} {:?}", id, times, now.elapsed());
            Ok(())
        }
    }
}

impl Server for CommandTraitImpl {
    fn run(&mut self, params: RunParams, results: RunResults) -> Promise<(), capnp::Error> {
        Promise::from_future(self.run_impl(params, results).map_err(Into::into))
    }

    fn name(&mut self, _: NameParams, mut results: NameResults) -> Promise<(), capnp::Error> {
        let cmd = self.cmd.clone();
        Promise::from_future(async move {
            let name = cmd.lock().await.name();
            results.get().set_name(name);
            Ok(())
        })
    }

    fn inputs(&mut self, _: InputsParams, mut results: InputsResults) -> Promise<(), capnp::Error> {
        let cmd = self.cmd.clone();
        Promise::from_future(
            async move {
                let inputs = cmd.lock().await.inputs();
                let inputs = simd_json::to_vec(&inputs).context(SimdJsonSnafu {
                    context: "serialize inputs description",
                })?;
                results.get().set_inputs(&inputs);
                Ok::<_, Error>(())
            }
            .map_err(Into::into),
        )
    }

    fn outputs(
        &mut self,
        _: OutputsParams,
        mut results: OutputsResults,
    ) -> Promise<(), capnp::Error> {
        let cmd = self.cmd.clone();
        Promise::from_future(
            async move {
                let outputs = cmd.lock().await.outputs();
                let outputs = simd_json::to_vec(&outputs).context(SimdJsonSnafu {
                    context: "serialize outputs description",
                })?;
                results.get().set_outputs(&outputs);
                Ok::<_, Error>(())
            }
            .map_err(Into::into),
        )
    }

    fn instruction_info(
        &mut self,
        _: InstructionInfoParams,
        mut results: InstructionInfoResults,
    ) -> Promise<(), capnp::Error> {
        let cmd = self.cmd.clone();
        Promise::from_future(
            async move {
                let info = cmd.lock().await.instruction_info();
                let info = simd_json::to_vec(&info).context(SimdJsonSnafu {
                    context: "serialize instruction info",
                })?;
                results.get().set_info(&info);
                Ok::<_, Error>(())
            }
            .map_err(Into::into),
        )
    }

    fn permissions(
        &mut self,
        _: PermissionsParams,
        mut results: PermissionsResults,
    ) -> Promise<(), capnp::Error> {
        let cmd = self.cmd.clone();
        Promise::from_future(
            async move {
                let perm = cmd.lock().await.permissions();
                let perm = simd_json::to_vec(&perm).context(SimdJsonSnafu {
                    context: "serialize permissions",
                })?;
                results.get().set_permissions(&perm);
                Ok::<_, Error>(())
            }
            .map_err(Into::into),
        )
    }
}
