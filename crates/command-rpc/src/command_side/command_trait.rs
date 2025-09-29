use capnp::capability::Promise;
use flow_lib::{
    Value,
    command::CommandTrait,
    context::{
        CommandContext, CommandContextData, FlowServices, FlowSetServices, execute, get_jwt,
    },
    flow_run_events::DEFAULT_LOG_FILTER,
    utils::tower_client::unimplemented_svc,
    value::{
        self,
        bincode_impl::{map_from_bincode, map_to_bincode},
    },
};
use futures::TryFutureExt;
use std::{
    rc::Rc,
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use tracing::Instrument;

pub use crate::command_capnp::command_trait::*;
use crate::{anyhow2capnp, make_sync::MakeSync, tracing::TrackFlowRun};

pub fn new_client(cmd: Box<dyn CommandTrait>, tracker: TrackFlowRun) -> Client {
    capnp_rpc::new_client(CommandTraitImpl {
        cmd: Rc::new(Mutex::new(cmd)),
        tracker,
    })
}

struct CommandTraitImpl {
    cmd: Rc<Mutex<Box<dyn CommandTrait>>>,
    tracker: TrackFlowRun,
}

fn parse_inputs(params: run_params::Reader<'_>) -> Result<value::Map, anyhow::Error> {
    let inputs = params.get_inputs()?;
    Ok(map_from_bincode(inputs)?)
}

// TODO: old flow-lib code use reqwest client with 30 secs timeout
pub(crate) static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(Default::default);

impl CommandTraitImpl {
    fn run_impl(
        &mut self,
        params: RunParams,
        mut results: RunResults,
    ) -> impl Future<Output = Result<(), anyhow::Error>> + 'static {
        let cmd = self.cmd.clone();
        let tracker = self.tracker.clone();
        async move {
            let now = Instant::now();
            let params = params.get()?;
            let inputs = parse_inputs(params)?;
            let context = params.get_ctx()?;
            let resp = context.data_request().send().promise.await?;
            let data = resp.get()?.get_data()?;
            let value = Value::from_bincode(data)?;
            let data: CommandContextData = value::from_value(value)?;
            let run_id = data.flow.flow_run_id;
            let node_id = data.node_id;
            let times = data.times;
            let filter = data
                .flow
                .environment
                .get("RUST_LOG")
                .cloned()
                .unwrap_or_else(|| DEFAULT_LOG_FILTER.to_owned());
            let (span, node_log) = tracker.enter(run_id, &filter, node_id, times, context.clone());
            let ctx = CommandContext::builder()
                .execute(execute::Svc::new(MakeSync::new(context.clone())))
                .get_jwt(get_jwt::Svc::new(MakeSync::new(context.clone())))
                .flow(FlowServices {
                    signer: unimplemented_svc(),
                    set: FlowSetServices {
                        http: HTTP_CLIENT.clone(),
                        solana_client: Arc::new(
                            data.flow.set.solana.build_client(Some(HTTP_CLIENT.clone())),
                        ),
                        extensions: Arc::new(Default::default()),
                        api_input: unimplemented_svc(),
                    },
                })
                .data(data)
                .node_log(node_log)
                .build();
            let id = *ctx.node_id();
            let times = *ctx.times();
            let cmd_lock = cmd.lock().await;
            let result = cmd_lock.run(ctx, inputs).instrument(span).await;
            tokio::task::spawn_local(async move {
                // TODO: without this delay, tracker get dropped before event are sent by tracing
                // is there a better way?
                tokio::time::sleep(Duration::from_secs(5)).await;
                tracker.exit(run_id, node_id, times);
            });
            results.get().set_output(&map_to_bincode(&result?)?);
            tracing::info!("ran {}:{} {:?}", id, times, now.elapsed());
            Ok(())
        }
    }
}

impl Server for CommandTraitImpl {
    fn run(&mut self, params: RunParams, results: RunResults) -> Promise<(), capnp::Error> {
        Promise::from_future(self.run_impl(params, results).map_err(anyhow2capnp))
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
                let inputs = simd_json::to_vec(&inputs)?;
                results.get().set_inputs(&inputs);
                Ok::<_, anyhow::Error>(())
            }
            .map_err(anyhow2capnp),
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
                let outputs = simd_json::to_vec(&outputs)?;
                results.get().set_outputs(&outputs);
                Ok::<_, anyhow::Error>(())
            }
            .map_err(anyhow2capnp),
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
                let info = simd_json::to_vec(&info)?;
                results.get().set_info(&info);
                Ok::<_, anyhow::Error>(())
            }
            .map_err(anyhow2capnp),
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
                let perm = simd_json::to_vec(&perm)?;
                results.get().set_permissions(&perm);
                Ok::<_, anyhow::Error>(())
            }
            .map_err(anyhow2capnp),
        )
    }
}
