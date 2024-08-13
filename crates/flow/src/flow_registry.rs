use crate::{
    command::{interflow, interflow_instructions},
    flow_graph::FlowRunResult,
    flow_run_events::{self, EventSender, NodeLog},
    FlowGraph,
};
use chrono::Utc;
use flow_lib::{
    config::{
        client::{BundlingMode, ClientConfig, FlowRunOrigin, PartialConfig},
        Endpoints,
    },
    context::{execute, get_jwt, signer, User},
    solana::{ExecuteOn, SolanaActionConfig},
    utils::TowerClient,
    CommandType, FlowConfig, FlowId, FlowRunId, NodeId, SolanaClientConfig, UserId, ValueSet,
};
use futures::channel::oneshot;
use hashbrown::HashMap;
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};
use thiserror::Error as ThisError;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::Instrument;
use utils::actix_service::ActixService;

pub const MAX_CALL_DEPTH: u32 = 32;

#[derive(Debug, ThisError)]
pub enum StopError {
    #[error("flow not found")]
    NotFound,
    #[error("forbidden")]
    Forbidden,
}

/// A collection of flows config to run together
#[derive(Clone)]
pub struct FlowRegistry {
    flows: Arc<HashMap<FlowId, ClientConfig>>,
    pub(crate) flow_owner: User,
    pub(crate) started_by: User,
    shared_with: Vec<UserId>,
    signers_info: JsonValue,
    endpoints: Endpoints,

    depth: u32,

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
        let signer = signer::unimplemented_svc();
        let new_flow_run = new_flow_run::unimplemented_svc();
        let get_previous_values = get_previous_values::unimplemented_svc();
        let token = get_jwt::unimplemented_svc();
        Self {
            depth: 0,
            flow_owner: User::new(UserId::nil()),
            started_by: User::new(UserId::nil()),
            shared_with: <_>::default(),
            flows: <_>::default(),
            endpoints: <_>::default(),
            signers_info: <_>::default(),
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
                .call_mut(get_flow::Request { user_id, flow_id })
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
                    .into())
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
            match (req.ctx.extensions.get::<EventSender>(), &req.ctx.command) {
                (Some(tx), Some(cmd)) => {
                    let tx1 = tx.clone();
                    let tx2 = tx.clone();
                    let node_id = cmd.node_id;
                    let times = cmd.times;
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
                            let module = if let (Some(line), Some(position)) =
                                (pos.line(), pos.position())
                            {
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
                }
                _ => {
                    engine
                        .on_print(|s| {
                            tracing::info!("rhai: {}", s);
                        })
                        .on_debug(move |s, _, pos| {
                            tracing::info!("rhai: {}, at {}", s, pos);
                        });
                }
            }
            match req.ctx.extensions.get::<CancellationToken>().cloned() {
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

impl FlowRegistry {
    pub async fn new(
        flow_owner: User,
        started_by: User,
        shared_with: Vec<UserId>,
        entrypoint: FlowId,
        (signer, signers_info): (signer::Svc, JsonValue),
        new_flow_run: new_flow_run::Svc,
        get_flow: get_flow::Svc,
        get_previous_values: get_previous_values::Svc,
        token: get_jwt::Svc,
        environment: HashMap<String, String>,
        endpoints: Endpoints,
    ) -> crate::Result<Self> {
        let flows = get_all_flows(entrypoint, flow_owner.id, get_flow, environment).await?;
        Ok(Self {
            depth: 0,
            flow_owner,
            started_by,
            shared_with,
            flows: Arc::new(flows),
            signer,
            signers_info,
            new_flow_run,
            get_previous_values,
            parent_flow_execute: None,
            token,
            endpoints,
            rhai_permit: Arc::new(Semaphore::new(1)),
            rhai_tx: <_>::default(),
            rpc_server: srpc::Server::start_http_server()
                .inspect_err(|error| tracing::info!("srpc error: {}", error))
                .ok(),
        })
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

    pub async fn from_actix(
        flow_owner: User,
        started_by: User,
        shared_with: Vec<UserId>,
        entrypoint: FlowId,
        (signer, signers_info): (actix::Recipient<signer::SignatureRequest>, JsonValue),
        new_flow_run: actix::Recipient<new_flow_run::Request>,
        get_flow: actix::Recipient<get_flow::Request>,
        get_previous_values: actix::Recipient<get_previous_values::Request>,
        token: actix::Recipient<get_jwt::Request>,
        environment: HashMap<String, String>,
        endpoints: Endpoints,
    ) -> crate::Result<Self> {
        Self::new(
            flow_owner,
            started_by,
            shared_with,
            entrypoint,
            (
                TowerClient::from_service(ActixService::from(signer), signer::Error::Worker, 16),
                signers_info,
            ),
            TowerClient::from_service(
                ActixService::from(new_flow_run),
                new_flow_run::Error::Worker,
                16,
            ),
            TowerClient::from_service(ActixService::from(get_flow), get_flow::Error::Worker, 16),
            TowerClient::from_service(
                ActixService::from(get_previous_values),
                get_previous_values::Error::Worker,
                16,
            ),
            TowerClient::from_service(
                tower::retry::Retry::new(
                    get_jwt::RetryPolicy::default(),
                    ActixService::from(token),
                ),
                get_jwt::Error::worker,
                16,
            ),
            environment,
            endpoints,
        )
        .await
    }

    pub async fn start(
        &self,
        flow_id: FlowId,
        inputs: ValueSet,
        partial_config: Option<PartialConfig>,
        collect_instructions: bool,
        action_config: Option<SolanaActionConfig>,
        origin: FlowRunOrigin,
        solana_client: Option<SolanaClientConfig>,
        parent_flow_execute: Option<execute::Svc>,
    ) -> Result<(FlowRunId, tokio::task::JoinHandle<FlowRunResult>), new_flow_run::Error> {
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
            .call_ref(new_flow_run::Request {
                user_id: self.flow_owner.id,
                shared_with: self.shared_with.clone(),
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
                    tracing::info!("flow {} no instruction_info: {}", id, error);
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
                    .call_mut(get_previous_values::Request { user_id, nodes })
                    .await?
                    .values
            } else {
                <_>::default()
            };

            let join_handle = tokio::spawn(
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
    use flow_lib::{command::CommandError, Context, ValueSet};
    use futures::channel::oneshot;
    use std::sync::Arc;

    pub type ChannelMessage = (Request, oneshot::Sender<Result<Response, Error>>);

    pub struct Request {
        pub command: Arc<rhai_script::Command>,
        pub ctx: Context,
        pub input: ValueSet,
    }

    pub type Response = ValueSet;

    pub type Error = CommandError;
}

pub mod new_flow_run {
    use crate::{
        flow_graph::StopSignal,
        flow_run_events::{Event, EventSender},
    };
    use flow_lib::{
        config::client::ClientConfig, utils::TowerClient, BoxError, FlowRunId, UserId, ValueSet,
    };
    use futures::stream::BoxStream;
    use thiserror::Error as ThisError;

    pub type Svc = TowerClient<Request, Response, Error>;

    pub struct Request {
        pub user_id: UserId,
        pub config: ClientConfig,
        pub shared_with: Vec<UserId>,
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
        Worker(tower::BoxError),
        #[error(transparent)]
        MailBox(#[from] actix::MailboxError),
        #[error(transparent)]
        Other(#[from] BoxError),
    }

    impl Error {
        pub fn other<E: Into<BoxError>>(e: E) -> Self {
            Self::Other(e.into())
        }
    }

    pub struct Response {
        pub flow_run_id: FlowRunId,
        pub stop_signal: StopSignal,
        pub stop_shared_signal: StopSignal,
        pub span: tracing::Span,
    }

    pub fn unimplemented_svc() -> Svc {
        Svc::unimplemented(|| BoxError::from("unimplemented").into(), Error::Worker)
    }
}

pub mod get_flow {
    use flow_lib::{
        config::client::ClientConfig, utils::TowerClient, BoxError, FlowId, NodeId, UserId,
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
    use flow_lib::{utils::TowerClient, BoxError, FlowRunId, NodeId, UserId};
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
        Worker(tower::BoxError),
        #[error(transparent)]
        MailBox(#[from] actix::MailboxError),
        #[error(transparent)]
        Other(#[from] BoxError),
    }

    pub fn unimplemented_svc() -> Svc {
        Svc::unimplemented(|| BoxError::from("unimplemented").into(), Error::Worker)
    }
}
