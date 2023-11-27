use crate::{
    command::{interflow, interflow_instructions},
    flow_graph::FlowRunResult,
    flow_run_events, FlowGraph,
};
use flow_lib::{
    config::{
        client::{BundlingMode, ClientConfig, FlowRunOrigin, PartialConfig},
        Endpoints,
    },
    context::{get_jwt, signer, User},
    utils::TowerClient,
    CommandType, FlowConfig, FlowId, FlowRunId, NodeId, UserId, ValueSet,
};
use hashbrown::HashMap;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use thiserror::Error as ThisError;
use tracing::instrument::WithSubscriber;
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
    depth: u32,
    pub(crate) flow_owner: User,
    pub(crate) started_by: User,
    shared_with: Vec<UserId>,
    flows: Arc<HashMap<FlowId, ClientConfig>>,
    signers_info: JsonValue,
    endpoints: Endpoints,
    pub(crate) signer: signer::Svc,
    pub(crate) token: get_jwt::Svc,
    new_flow_run: new_flow_run::Svc,
    get_previous_values: get_previous_values::Svc,
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
            flows: Arc::new(HashMap::new()),
            endpoints: <_>::default(),
            signers_info: <_>::default(),
            signer,
            token,
            new_flow_run,
            get_previous_values,
        }
    }
}

async fn get_all_flows(
    entrypoint: FlowId,
    user_id: UserId,
    mut get_flow: get_flow::Svc,
    environment: HashMap<String, String>,
) -> Result<HashMap<FlowId, ClientConfig>, get_flow::Error> {
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
                    })
                }
            }
        }
        flows.insert(flow_id, config);
    }

    Ok(flows)
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
    ) -> Result<Self, get_flow::Error> {
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
            token,
            endpoints,
        })
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
    ) -> Result<Self, get_flow::Error> {
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
        origin: FlowRunOrigin,
    ) -> Result<(FlowRunId, tokio::task::JoinHandle<FlowRunResult>), new_flow_run::Error> {
        let config = self
            .flows
            .get(&flow_id)
            .ok_or(new_flow_run::Error::NotFound)?;

        if self.depth >= MAX_CALL_DEPTH {
            return Err(new_flow_run::Error::MaxDepthReached);
        }
        let this = Self {
            depth: self.depth + 1,
            ..self.clone()
        };

        let (tx, rx) = futures::channel::mpsc::unbounded();
        let run = self
            .new_flow_run
            .call_ref(new_flow_run::Request {
                user_id: self.flow_owner.id,
                shared_with: self.shared_with.clone(),
                config: ClientConfig {
                    call_depth: self.depth,
                    origin,
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
                stream: Box::pin(rx),
            })
            .await?;

        let flow_run_id = run.flow_run_id;
        let stop = run.stop_signal;

        let subscriber = flow_run_events::build_tracing_subscriber(
            tx.clone(),
            config.environment.get("RUST_LOG").map(String::as_str),
        );
        async move {
            let mut get_previous_values_svc = this.get_previous_values.clone();
            let user_id = this.flow_owner.id;
            let mut flow_config = FlowConfig::new(config.clone());
            flow_config.ctx.endpoints = this.endpoints.clone();
            let mut flow = FlowGraph::from_cfg(flow_config, this, partial_config.as_ref()).await?;

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
                    flow.run(tx, flow_run_id, inputs, stop, previous_values)
                        .await
                }
                .with_current_subscriber(),
            );

            Ok((flow_run_id, join_handle))
        }
        .with_subscriber(subscriber)
        .await
    }
}

pub mod new_flow_run {
    use crate::{flow_graph::StopSignal, flow_run_events};
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
        pub stream: BoxStream<'static, flow_run_events::Event>,
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
