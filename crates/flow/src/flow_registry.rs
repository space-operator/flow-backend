use crate::{
    FlowGraph,
    command::{interflow, interflow_instructions},
    flow_graph::FlowRunResult,
    flow_run_events::{self, EventSender, NodeLog},
    flow_set::DeploymentId,
};
use chrono::Utc;
use flow_lib::{
    CommandType, FlowConfig, FlowId, FlowRunId, NodeId, SolanaClientConfig, UserId, ValueSet,
    config::{
        Endpoints,
        client::{BundlingMode, ClientConfig, FlowRunOrigin, PartialConfig},
    },
    context::{User, api_input, execute, get_jwt, signer},
    solana::{ExecuteOn, Pubkey, SolanaActionConfig},
    utils::{
        TowerClient,
        tower_client::{CommonErrorExt, unimplemented_svc},
    },
};
use futures::channel::oneshot;
use hashbrown::HashMap;
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex, OnceLock};
use thiserror::Error as ThisError;
use tokio::{
    runtime::Handle,
    sync::{Semaphore, mpsc},
    task::spawn_local,
};
use tokio_util::sync::CancellationToken;
use tower::{Service, ServiceExt};
use tracing::Instrument;
use utils::actix_service::ActixService;

pub const MAX_CALL_DEPTH: u32 = 1024;

#[derive(Debug, ThisError)]
pub enum StopError {
    #[error("flow not found")]
    NotFound,
    #[error("forbidden")]
    Forbidden,
}

#[derive(Default)]
pub struct StartFlowOptions {
    pub partial_config: Option<PartialConfig>,
    pub collect_instructions: bool,
    pub action_identity: Option<Pubkey>,
    pub action_config: Option<SolanaActionConfig>,
    pub fees: Vec<(Pubkey, u64)>,
    pub origin: FlowRunOrigin,
    pub solana_client: Option<SolanaClientConfig>,
    pub parent_flow_execute: Option<execute::Svc>,
    pub deployment_id: Option<DeploymentId>,
}

/// A collection of flows config to run together
#[derive(Clone, bon::Builder)]
pub struct FlowRegistry {
    flows: Arc<HashMap<FlowId, ClientConfig>>,
    pub(crate) flow_owner: User,
    pub(crate) started_by: User,
    shared_with: Vec<UserId>,
    signers_info: JsonValue,
    endpoints: Endpoints,

    depth: u32,

    pub(crate) api_input: api_input::Svc,
    pub(crate) signer: signer::Svc,
    pub(crate) token: get_jwt::Svc,
    new_flow_run: new_flow_run::Svc,
    get_previous_values: get_previous_values::Svc,
    pub(crate) parent_flow_execute: Option<execute::Svc>,

    pub(crate) rhai_permit: Arc<Semaphore>,
    rhai_tx: Arc<Mutex<Option<crossbeam_channel::Sender<run_rhai::ChannelMessage>>>>,

    pub(crate) rpc_server: Option<actix::Addr<srpc::Server>>,
}

impl Default for FlowRegistry {
    fn default() -> Self {
        let signer = unimplemented_svc();
        let new_flow_run = unimplemented_svc();
        let get_previous_values = unimplemented_svc();
        let token = unimplemented_svc();
        let api_input = unimplemented_svc();

        Self {
            depth: 0,
            flow_owner: User::new(UserId::nil()),
            started_by: User::new(UserId::nil()),
            shared_with: <_>::default(),
            flows: <_>::default(),
            endpoints: <_>::default(),
            signers_info: <_>::default(),
            api_input,
            signer,
            token,
            new_flow_run,
            get_previous_values,
            parent_flow_execute: None,
            rhai_permit: Arc::new(Semaphore::new(1)),
            rhai_tx: <_>::default(),
            rpc_server: None, // TODO: try this
        }
    }
}

async fn get_all_flows(
    entrypoint: FlowId,
    user_id: UserId,
    mut get_flow: get_flow::Svc,
    environment: HashMap<String, String>,
) -> crate::Result<HashMap<FlowId, ClientConfig>> {
    let mut flows = HashMap::new();

    let mut queue = [entrypoint].to_vec();
    while let Some(flow_id) = queue.pop() {
        let config = {
            let mut config = get_flow
                .ready()
                .await?
                .call(get_flow::Request { user_id, flow_id })
                .await?
                .config;
            for (k, v) in &environment {
                config
                    .environment
                    .entry(k.clone())
                    .or_insert_with(|| v.clone());
            }
            config
        };
        let interflow_nodes = config
            .nodes
            .iter()
            .filter(|n| {
                n.data.r#type == CommandType::Native
                    && (n.data.node_id == interflow::INTERFLOW
                        || n.data.node_id == interflow_instructions::INTERFLOW_INSTRUCTIONS)
            })
            .map(|n| (n.id, interflow::get_interflow_id(&n.data)));
        for (node_id, result) in interflow_nodes {
            match result {
                Ok(id) => {
                    if id != flow_id && !flows.contains_key(&id) {
                        queue.push(id);
                    }
                }
                Err(error) => {
                    return Err(get_flow::Error::InvalidInferflow {
                        flow_id,
                        node_id,
                        error,
                    }
                    .into());
                }
            }
        }
        flows.insert(flow_id, config);
    }

    let mut info = HashMap::new();
    for (id, config) in flows.iter_mut() {
        if config.instructions_bundling != BundlingMode::Off {
            let g = FlowGraph::from_cfg(
                FlowConfig::new(config.clone()),
                FlowRegistry::default(),
                None,
            )
            .await?;
            config.interflow_instruction_info = g
                .get_interflow_instruction_info()
                .map_err(|error| error.to_string());
            if let Ok(i) = config.interflow_instruction_info.as_ref() {
                info.insert(*id, i.clone());
            }
        }
    }

    for (parent_flow, config) in flows.iter_mut() {
        let interflows = config.nodes.iter_mut().filter(|n| {
            n.data.r#type == CommandType::Native && n.data.node_id == interflow::INTERFLOW
        });

        for n in interflows {
            let flow_id = interflow::get_interflow_id(&n.data).map_err(|error| {
                get_flow::Error::InvalidInferflow {
                    flow_id: *parent_flow,
                    node_id: n.id,
                    error,
                }
            })?;
            n.data.instruction_info = info.get(&flow_id).cloned();
        }
    }

    Ok(flows)
}

fn spawn_rhai_thread(rx: crossbeam_channel::Receiver<run_rhai::ChannelMessage>) {
    tokio::task::spawn_blocking(move || {
        let mut engine = rhai_script::setup_engine();
        while let Ok((req, tx)) = rx.recv() {
            if let Some(tx) = req.ctx.get::<EventSender>() {
                let tx1 = tx.clone();
                let tx2 = tx.clone();
                let node_id = *req.ctx.node_id();
                let times = *req.ctx.times();
                engine
                    .on_print(move |s| {
                        tx1.unbounded_send(
                            NodeLog {
                                time: Utc::now(),
                                node_id,
                                times,
                                level: flow_run_events::LogLevel::Info,
                                module: None,
                                content: s.to_owned(),
                            }
                            .into(),
                        )
                        .ok();
                    })
                    .on_debug(move |s, _, pos| {
                        let module =
                            if let (Some(line), Some(position)) = (pos.line(), pos.position()) {
                                Some(format!("script.rhai:{}:{}", line, position))
                            } else {
                                None
                            };
                        tx2.unbounded_send(
                            NodeLog {
                                time: Utc::now(),
                                node_id,
                                times,
                                level: flow_run_events::LogLevel::Debug,
                                module,
                                content: s.to_owned(),
                            }
                            .into(),
                        )
                        .ok();
                    });
            } else {
                engine
                    .on_print(|s| {
                        tracing::info!("rhai: {}", s);
                    })
                    .on_debug(move |s, _, pos| {
                        tracing::info!("rhai: {}, at {}", s, pos);
                    });
            }
            match req.ctx.get::<CancellationToken>().cloned() {
                Some(stop_token) => {
                    engine.on_progress(move |c| {
                        (c % 4096 == 0 && stop_token.is_cancelled()).then(|| "canceled".into())
                    });
                }
                None => {
                    engine.on_progress(|_| None);
                }
            }
            let result = req.command.run(&mut engine, req.ctx, req.input);
            if tx.send(result).is_err() {
                tracing::debug!("command stopped waiting");
            }
        }
    });
}

pub struct StartReq {
    flow_id: FlowId,
    inputs: value::Map,
    options: StartFlowOptions,
    recv: oneshot::Sender<StartResp>,
}

type StartResp = Result<(FlowRunId, tokio::task::JoinHandle<FlowRunResult>), new_flow_run::Error>;

pub struct FlowRegistryHandle {
    tx: mpsc::Sender<StartReq>,
}

impl FlowRegistryHandle {
    pub async fn start(
        &self,
        flow_id: FlowId,
        inputs: ValueSet,
        options: StartFlowOptions,
    ) -> Result<(FlowRunId, tokio::task::JoinHandle<FlowRunResult>), new_flow_run::Error> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(StartReq {
                flow_id,
                inputs,
                options,
                recv: tx,
            })
            .await
            .map_err(new_flow_run::Error::other)?;
        rx.await.map_err(new_flow_run::Error::other)?
    }
}

impl FlowRegistry {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        flow_owner: User,
        started_by: User,
        shared_with: Vec<UserId>,
        entrypoint: FlowId,
        api_input: api_input::Svc,
        (signer, signers_info): (signer::Svc, JsonValue),
        new_flow_run: new_flow_run::Svc,
        get_flow: get_flow::Svc,
        get_previous_values: get_previous_values::Svc,
        token: get_jwt::Svc,
        environment: HashMap<String, String>,
        endpoints: Endpoints,
    ) -> crate::Result<FlowRegistryHandle> {
        let (tx, mut rx) = mpsc::channel::<StartReq>(1);
        std::thread::spawn(move || {
            let rt = tokio::runtime::LocalRuntime::new().unwrap();
            rt.block_on(async move {
                let flows = get_all_flows(entrypoint, flow_owner.id, get_flow, environment).await?;
                let registry = Self {
                    depth: 0,
                    flow_owner,
                    started_by,
                    shared_with,
                    flows: Arc::new(flows),
                    signer,
                    signers_info,
                    new_flow_run,
                    api_input,
                    get_previous_values,
                    parent_flow_execute: None,
                    token,
                    endpoints,
                    rhai_permit: Arc::new(Semaphore::new(1)),
                    rhai_tx: <_>::default(),
                    rpc_server: srpc::Server::start_http_server()
                        .inspect_err(|error| tracing::error!("srpc error: {}", error))
                        .ok(),
                };

                while let Some(req) = rx.recv().await {
                    req.recv
                        .send(registry.start(req.flow_id, req.inputs, req.options).await)
                        .ok();
                }

                Ok::<(), crate::Error>(())
            })
        });
        Ok(FlowRegistryHandle { tx })
    }

    pub async fn run_rhai(
        &self,
        req: run_rhai::Request,
    ) -> Result<run_rhai::Response, run_rhai::Error> {
        let worker = {
            let mut tx = self.rhai_tx.lock().unwrap();
            if tx.is_none() {
                let (new_tx, rx) = crossbeam_channel::unbounded();
                spawn_rhai_thread(rx);
                *tx = Some(new_tx.clone());
            }
            tx.clone().unwrap()
        };
        let (tx, rx) = oneshot::channel();
        worker
            .send((req, tx))
            .map_err(|_| run_rhai::Error::msg("rhai worker stopped"))?;
        rx.await
            .map_err(|_| run_rhai::Error::msg("rhai worker stopped"))?
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn from_actix(
        flow_owner: User,
        started_by: User,
        shared_with: Vec<UserId>,
        entrypoint: FlowId,
        api_input: api_input::Svc,
        (signer, signers_info): (actix::Recipient<signer::SignatureRequest>, JsonValue),
        new_flow_run: actix::Recipient<new_flow_run::Request>,
        get_flow: actix::Recipient<get_flow::Request>,
        get_previous_values: actix::Recipient<get_previous_values::Request>,
        token: actix::Recipient<get_jwt::Request>,
        environment: HashMap<String, String>,
        endpoints: Endpoints,
    ) -> crate::Result<FlowRegistryHandle> {
        Self::new(
            flow_owner,
            started_by,
            shared_with,
            entrypoint,
            api_input,
            (TowerClient::new(ActixService::from(signer)), signers_info),
            TowerClient::new(ActixService::from(new_flow_run)),
            TowerClient::new(ActixService::from(get_flow)),
            TowerClient::new(ActixService::from(get_previous_values)),
            TowerClient::new(tower::retry::Retry::new(
                get_jwt::RetryPolicy::default(),
                ActixService::from(token),
            )),
            environment,
            endpoints,
        )
        .await
    }

    pub async fn start(
        &self,
        flow_id: FlowId,
        inputs: ValueSet,
        options: StartFlowOptions,
    ) -> Result<(FlowRunId, tokio::task::JoinHandle<FlowRunResult>), new_flow_run::Error> {
        let StartFlowOptions {
            partial_config,
            collect_instructions,
            action_identity,
            action_config,
            fees,
            origin,
            solana_client,
            parent_flow_execute,
            deployment_id,
        } = options;
        let config = self
            .flows
            .get(&flow_id)
            .ok_or(new_flow_run::Error::NotFound)?;
        let solana_client = solana_client.unwrap_or(config.sol_network.clone().into());

        if self.depth >= MAX_CALL_DEPTH {
            return Err(new_flow_run::Error::MaxDepthReached);
        }
        let this = Self {
            depth: self.depth + 1,
            parent_flow_execute,
            ..self.clone()
        };

        let (tx, rx) = flow_run_events::channel();
        let run = self
            .new_flow_run
            .clone()
            .ready()
            .await?
            .call(new_flow_run::Request {
                user_id: self.flow_owner.id,
                shared_with: self.shared_with.clone(),
                deployment_id,
                config: ClientConfig {
                    call_depth: self.depth,
                    origin,
                    sol_network: solana_client.clone().into(),
                    collect_instructions,
                    partial_config: partial_config.clone(),
                    instructions_bundling: if collect_instructions
                        && matches!(config.instructions_bundling, BundlingMode::Off)
                    {
                        BundlingMode::Automatic
                    } else {
                        config.instructions_bundling.clone()
                    },
                    signers: self.signers_info.clone(),
                    ..config.clone()
                },
                inputs: inputs.clone(),
                tx: tx.clone(),
                stream: Box::pin(rx),
            })
            .await?;

        let flow_run_id = run.flow_run_id;
        let stop = run.stop_signal;
        let stop_shared = run.stop_shared_signal;

        async move {
            this.flows.iter().for_each(|(id, flow)| {
                if let Err(error) = &flow.interflow_instruction_info {
                    tracing::debug!("flow {} no instruction_info: {}", id, error);
                }
            });

            let mut get_previous_values_svc = this.get_previous_values.clone();
            let user_id = this.flow_owner.id;
            let mut flow_config = FlowConfig::new(config.clone());
            flow_config.ctx.endpoints = this.endpoints.clone();
            flow_config.ctx.solana_client = solana_client.clone();
            let mut flow = FlowGraph::from_cfg(flow_config, this, partial_config.as_ref()).await?;

            if let Some(config) = action_config {
                flow.tx_exec_config.execute_on = ExecuteOn::SolanaAction(config);
            }
            flow.action_identity = action_identity;
            flow.fees = fees;

            if collect_instructions {
                if let BundlingMode::Off = flow.mode {
                    flow.mode = BundlingMode::Automatic;
                }
                flow.output_instructions = true;
            }

            let nodes = flow.need_previous_outputs();
            let nodes = nodes
                .into_iter()
                .filter_map(|id| {
                    partial_config
                        .as_ref()
                        .and_then(|c| {
                            c.values_config
                                .nodes
                                .get(&id)
                                .copied()
                                .or(c.values_config.default_run_id)
                        })
                        .map(|run_id| (id, run_id))
                })
                .collect::<HashMap<NodeId, FlowRunId>>();
            let previous_values = if !nodes.is_empty() {
                get_previous_values_svc
                    .ready()
                    .await?
                    .call(get_previous_values::Request { user_id, nodes })
                    .await?
                    .values
            } else {
                <_>::default()
            };

            let join_handle = spawn_local(
                async move {
                    flow.run(tx, flow_run_id, inputs, stop, stop_shared, previous_values)
                        .await
                }
                .in_current_span(),
            );

            Ok((flow_run_id, join_handle))
        }
        .instrument(run.span)
        .await
    }
}

pub mod run_rhai {
    use flow_lib::{ValueSet, command::CommandError, context::CommandContext};
    use futures::channel::oneshot;
    use std::sync::Arc;

    pub type ChannelMessage = (Request, oneshot::Sender<Result<Response, Error>>);

    pub struct Request {
        pub command: Arc<rhai_script::Command>,
        pub ctx: CommandContext,
        pub input: ValueSet,
    }

    pub type Response = ValueSet;

    pub type Error = CommandError;
}

pub mod new_flow_run {
    use crate::{
        flow_graph::StopSignal,
        flow_run_events::{Event, EventSender},
        flow_set::DeploymentId,
    };
    use flow_lib::{
        FlowRunId, UserId, ValueSet,
        config::client::ClientConfig,
        utils::{TowerClient, tower_client::CommonError},
    };
    use futures::stream::BoxStream;
    use thiserror::Error as ThisError;

    pub type Svc = TowerClient<Request, Response, Error>;

    pub struct Request {
        pub user_id: UserId,
        pub config: ClientConfig,
        pub shared_with: Vec<UserId>,
        pub deployment_id: Option<DeploymentId>,
        pub inputs: ValueSet,
        pub tx: EventSender,
        pub stream: BoxStream<'static, Event>,
    }

    impl actix::Message for Request {
        type Result = Result<Response, Error>;
    }

    #[derive(ThisError, Debug)]
    pub enum Error {
        #[error("recursive depth reached")]
        MaxDepthReached,
        #[error("flow not found")]
        NotFound,
        #[error("unauthorized")]
        Unauthorized,
        #[error(transparent)]
        GetPreviousValues(#[from] super::get_previous_values::Error),
        #[error(transparent)]
        BuildFlow(#[from] crate::Error),
        #[error(transparent)]
        Common(#[from] CommonError),
    }

    impl From<actix::MailboxError> for Error {
        fn from(value: actix::MailboxError) -> Self {
            CommonError::from(value).into()
        }
    }

    pub struct Response {
        pub flow_run_id: FlowRunId,
        pub stop_signal: StopSignal,
        pub stop_shared_signal: StopSignal,
        pub span: tracing::Span,
    }
}

pub mod get_flow {
    use flow_lib::{
        BoxError, FlowId, NodeId, UserId, config::client::ClientConfig, utils::TowerClient,
    };
    use thiserror::Error as ThisError;

    pub type Svc = TowerClient<Request, Response, Error>;

    pub struct Request {
        pub user_id: UserId,
        pub flow_id: FlowId,
    }

    impl actix::Message for Request {
        type Result = Result<Response, Error>;
    }

    pub struct Response {
        pub config: ClientConfig,
    }

    #[derive(ThisError, Debug)]
    pub enum Error {
        #[error("flow not found")]
        NotFound,
        #[error("unauthorized")]
        Unauthorized,
        #[error(
            "parsing interflow failed, flow_id={}, node_id={}: {}",
            flow_id,
            node_id,
            error
        )]
        InvalidInferflow {
            flow_id: FlowId,
            node_id: NodeId,
            error: serde_json::Error,
        },
        #[error(transparent)]
        Worker(tower::BoxError),
        #[error(transparent)]
        MailBox(#[from] actix::MailboxError),
        #[error(transparent)]
        Other(#[from] BoxError),
    }
}

pub mod get_previous_values {
    use flow_lib::{
        FlowRunId, NodeId, UserId,
        utils::{TowerClient, tower_client::CommonError},
    };
    use hashbrown::HashMap;
    use thiserror::Error as ThisError;
    use value::Value;

    pub type Svc = TowerClient<Request, Response, Error>;

    pub struct Request {
        pub user_id: UserId,
        pub nodes: HashMap<NodeId, FlowRunId>,
    }

    impl actix::Message for Request {
        type Result = Result<Response, Error>;
    }

    pub struct Response {
        pub values: HashMap<NodeId, Vec<Value>>,
    }

    #[derive(ThisError, Debug)]
    pub enum Error {
        #[error("unauthorized")]
        Unauthorized,
        #[error(transparent)]
        Common(#[from] CommonError),
    }

    impl From<actix::MailboxError> for Error {
        fn from(value: actix::MailboxError) -> Self {
            CommonError::from(value).into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry() {
        FlowRegistry::default();
    }
}
