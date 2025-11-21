use crate::{
    FlowGraph,
    command::{interflow, interflow_instructions},
    flow_graph::FlowRunResult,
    flow_set::DeploymentId,
};
use chrono::Utc;
use command_rpc::flow_side::address_book::AddressBook;
use flow_lib::{
    CommandType, FlowConfig, FlowId, FlowRunId, NodeId, SolanaClientConfig, UserId,
    config::{
        Endpoints,
        client::{BundlingMode, ClientConfig, FlowRunOrigin, PartialConfig},
    },
    context::{Helius, User, api_input, execute, get_jwt, signer},
    flow_run_events::{self, EventSender, NodeLog},
    solana::{ExecuteOn, Pubkey, SolanaActionConfig},
    utils::tower_client::{CommonErrorExt, unimplemented_svc},
};
use futures::channel::oneshot;
use hashbrown::HashMap;
use serde_json::Value as JsonValue;
use std::sync::{Arc, OnceLock};
use thiserror::Error as ThisError;
use tokio::{
    sync::{Semaphore, mpsc},
    task::spawn_local,
};
use tokio_util::sync::CancellationToken;
use tower::{Service, ServiceExt, service_fn};
use tracing::Instrument;

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

#[derive(Clone)]
pub struct BackendServices {
    pub api_input: api_input::Svc,
    pub signer: signer::Svc,
    pub token: get_jwt::Svc,
    pub new_flow_run: new_flow_run::Svc,
    pub get_previous_values: get_previous_values::Svc,
    pub helius: Option<Arc<Helius>>,
}

impl BackendServices {
    fn unimplemented() -> Self {
        Self {
            api_input: unimplemented_svc(),
            signer: unimplemented_svc(),
            token: unimplemented_svc(),
            new_flow_run: unimplemented_svc(),
            get_previous_values: unimplemented_svc(),
            helius: None,
        }
    }
}

/// A collection of flows config to run together
#[derive(Clone, bon::Builder)]
pub struct FlowRegistry {
    pub(crate) flow_owner: User,
    pub(crate) started_by: User,
    shared_with: Vec<UserId>,
    signers_info: JsonValue,
    endpoints: Endpoints,

    flows: Arc<HashMap<FlowId, ClientConfig>>,

    depth: u32,

    pub(crate) backend: BackendServices,
    pub(crate) parent_flow_execute: Option<execute::Svc>,

    pub(crate) rhai_permit: Arc<Semaphore>,
    rhai_tx: Arc<OnceLock<crossbeam_channel::Sender<run_rhai::ChannelMessage>>>,

    pub(crate) rpc_server: Option<actix::Addr<srpc::Server>>,
    pub(crate) remotes: Option<AddressBook>,

    #[builder(default)]
    pub(crate) http: reqwest::Client,
}

impl Default for FlowRegistry {
    fn default() -> Self {
        Self {
            depth: 0,
            flow_owner: User::new(UserId::nil()),
            started_by: User::new(UserId::nil()),
            shared_with: <_>::default(),
            flows: <_>::default(),
            endpoints: <_>::default(),
            signers_info: <_>::default(),
            parent_flow_execute: None,
            backend: BackendServices::unimplemented(),
            rhai_permit: Arc::new(Semaphore::new(1)),
            rhai_tx: <_>::default(),
            rpc_server: None, // TODO: try this
            remotes: None,
            http: <_>::default(),
        }
    }
}

async fn get_all_flows<S>(
    entrypoint: FlowId,
    user_id: UserId,
    mut get_flow: S,
    environment: HashMap<String, String>,
) -> crate::Result<HashMap<FlowId, ClientConfig>>
where
    S: Service<get_flow::Request, Response = get_flow::Response, Error = get_flow::Error>,
{
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
                                Some(format!("script.rhai:{line}:{position}"))
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

#[bon::bon]
impl FlowRegistry {
    #[builder]
    pub async fn fetch<S>(
        entrypoint: FlowId,
        flow_owner: User,
        started_by: User,
        shared_with: Vec<UserId>,
        environment: HashMap<String, String>,
        endpoints: Endpoints,
        signers_info: JsonValue,
        remotes: Option<AddressBook>,
        backend: BackendServices,
        http: Option<reqwest::Client>,
        get_flow: S,
    ) -> crate::Result<Self>
    where
        S: Service<get_flow::Request, Response = get_flow::Response, Error = get_flow::Error>,
    {
        let flows = get_all_flows(entrypoint, flow_owner.id, get_flow, environment).await?;
        Ok(Self {
            depth: 0,
            flow_owner,
            started_by,
            shared_with,
            flows: Arc::new(flows),
            signers_info,
            parent_flow_execute: None,
            endpoints,
            backend,
            rhai_permit: Arc::new(Semaphore::new(1)),
            rhai_tx: <_>::default(),
            rpc_server: srpc::Server::start_http_server()
                .inspect_err(|error| tracing::error!("srpc error: {}", error))
                .ok(),
            remotes,
            http: http.unwrap_or_default(),
        })
    }
}

impl FlowRegistry {
    pub fn make_start_flow_svc(&self) -> start_flow::Svc {
        let mut this = self.clone();
        let (tx, mut rx) = mpsc::channel::<(
            start_flow::Request,
            oneshot::Sender<Result<start_flow::Response, start_flow::Error>>,
        )>(1);
        spawn_local(async move {
            while let Some((req, tx)) = rx.recv().await {
                let result = this.start(req.flow_id, req.inputs, req.options).await;
                tx.send(result).ok();
            }
        });
        start_flow::Svc::new(service_fn(move |req: start_flow::Request| {
            let tx = tx.clone();
            async move {
                let (resp, rx) = oneshot::channel();
                tx.send((req, resp))
                    .await
                    .map_err(new_flow_run::Error::other)?;
                rx.await.map_err(new_flow_run::Error::other)?
            }
        }))
    }

    pub fn make_run_rhai_svc(&self) -> run_rhai::Svc {
        let worker = self
            .rhai_tx
            .get_or_init(|| {
                let (new_tx, rx) = crossbeam_channel::unbounded();
                spawn_rhai_thread(rx);
                new_tx
            })
            .clone();
        let service = move |req: run_rhai::Request| {
            let worker = worker.clone();
            Box::pin(async move {
                let (tx, rx) = oneshot::channel();
                worker
                    .send((req, tx))
                    .map_err(|_| run_rhai::Error::msg("rhai worker stopped"))?;
                rx.await
                    .map_err(|_| run_rhai::Error::msg("rhai worker stopped"))?
            })
        };
        run_rhai::Svc::new(tower::service_fn(service))
    }

    fn fork_interflow(&self, parent_flow_execute: Option<execute::Svc>) -> Self {
        Self {
            depth: self.depth + 1,
            parent_flow_execute,
            ..self.clone()
        }
    }

    async fn start_inner(
        mut self,
        flow_id: FlowId,
        inputs: value::Map,
        options: StartFlowOptions,
    ) -> Result<(FlowRunId, tokio::task::JoinHandle<FlowRunResult>), new_flow_run::Error> {
        let flow = self
            .flows
            .get(&flow_id)
            .ok_or(new_flow_run::Error::NotFound)?
            .clone();
        let solana_client = options
            .solana_client
            .unwrap_or_else(|| flow.sol_network.clone().into());

        if self.depth >= MAX_CALL_DEPTH {
            return Err(new_flow_run::Error::MaxDepthReached);
        }

        let (tx, rx) = flow_run_events::channel();
        let new_flow_run::Response {
            flow_run_id,
            stop_signal,
            stop_shared_signal,
            span,
        } = self
            .backend
            .new_flow_run
            .ready()
            .await?
            .call(new_flow_run::Request {
                user_id: self.flow_owner.id,
                shared_with: self.shared_with.clone(),
                deployment_id: options.deployment_id,
                config: ClientConfig {
                    call_depth: self.depth,
                    origin: options.origin,
                    sol_network: solana_client.clone().into(),
                    collect_instructions: options.collect_instructions,
                    partial_config: options.partial_config.clone(),
                    instructions_bundling: if options.collect_instructions
                        && matches!(flow.instructions_bundling, BundlingMode::Off)
                    {
                        BundlingMode::Automatic
                    } else {
                        flow.instructions_bundling.clone()
                    },
                    signers: self.signers_info.clone(),
                    ..flow.clone()
                },
                inputs: inputs.clone(),
                tx: tx.clone(),
                stream: Box::pin(rx),
            })
            .await?;

        /*
        self.flows.iter().for_each(|(id, flow)| {
            if let Err(error) = &flow.interflow_instruction_info {
                tracing::debug!("flow {} no instruction_info: {}", id, error);
            }
        });
        */

        async {
            let mut flow_config = FlowConfig::new(flow.clone());
            flow_config.ctx.endpoints = self.endpoints.clone();
            flow_config.ctx.solana_client = solana_client.clone();
            let mut flow =
                FlowGraph::from_cfg(flow_config, self.clone(), options.partial_config.as_ref())
                    .await?;

            if let Some(config) = options.action_config {
                flow.tx_exec_config.execute_on = ExecuteOn::SolanaAction(config);
            }
            flow.action_identity = options.action_identity;
            flow.fees = options.fees;

            if options.collect_instructions {
                if let BundlingMode::Off = flow.mode {
                    flow.mode = BundlingMode::Automatic;
                }
                flow.output_instructions = true;
            }

            let previous_values = {
                let nodes = flow.need_previous_outputs();
                let nodes = nodes
                    .into_iter()
                    .filter_map(|id| {
                        options
                            .partial_config
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

                if !nodes.is_empty() {
                    self.backend
                        .get_previous_values
                        .ready()
                        .await?
                        .call(get_previous_values::Request {
                            user_id: self.flow_owner.id,
                            nodes,
                        })
                        .await?
                        .values
                } else {
                    <_>::default()
                }
            };

            let task = async move {
                flow.run(
                    tx,
                    flow_run_id,
                    inputs,
                    stop_signal,
                    stop_shared_signal,
                    previous_values,
                )
                .await
            }
            .in_current_span();
            let join_handle = spawn_local(task);

            Ok((flow_run_id, join_handle))
        }
        .instrument(span)
        .await
    }

    pub async fn start(
        &mut self,
        flow_id: FlowId,
        inputs: value::Map,
        options: StartFlowOptions,
    ) -> Result<(FlowRunId, tokio::task::JoinHandle<FlowRunResult>), new_flow_run::Error> {
        self.fork_interflow(options.parent_flow_execute.clone())
            .start_inner(flow_id, inputs, options)
            .await
    }
}

pub mod run_rhai {
    use flow_lib::{ValueSet, command::CommandError, context::CommandContext, utils::TowerClient};
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

    pub type Svc = TowerClient<Request, Response, Error>;
}

pub mod new_flow_run {
    use crate::{flow_graph::StopSignal, flow_set::DeploymentId};
    use flow_lib::{
        FlowRunId, UserId, ValueSet,
        config::client::ClientConfig,
        flow_run_events::{Event, EventSender},
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
        pub tx: EventSender, // only used to send log
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

pub mod start_flow {
    use crate::flow_graph::FlowRunResult;

    use super::{StartFlowOptions, new_flow_run};
    use flow_lib::{FlowId, FlowRunId, utils::TowerClient};

    pub type Svc = TowerClient<Request, Response, Error>;

    pub struct Request {
        pub flow_id: FlowId,
        pub inputs: value::Map,
        pub options: StartFlowOptions,
    }

    pub type Response = (FlowRunId, tokio::task::JoinHandle<FlowRunResult>);

    pub type Error = new_flow_run::Error;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry() {
        FlowRegistry::default();
    }
}
