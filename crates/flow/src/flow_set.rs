use flow_lib::{
    config::{
        client::{ClientConfig, FlowRow, FlowRunOrigin},
        Endpoints,
    },
    context::{execute, get_jwt, signer},
    solana::{Pubkey, SolanaActionConfig},
    CommandType, FlowId, FlowRunId, NodeId, SolanaClientConfig, User, UserId, ValueSet,
};
use getset::Getters;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};
use tokio::{sync::Semaphore, task::JoinHandle};
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    command::{interflow, interflow_instructions},
    flow_graph::FlowRunResult,
    flow_registry::{get_previous_values, new_flow_run, run_rhai, FlowRegistry, StartFlowOptions},
};

/// Who can start flows
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum StartPermission {
    /// Only flow owner can start
    Owner,
    /// Any authenticated user
    Authenticated,
    /// Any unauthenticated user
    Anonymous,
}

#[derive(bon::Builder, Clone, Debug)]
pub struct Flow {
    pub row: FlowRow,
}

impl Flow {
    pub fn start_permission(&self) -> StartPermission {
        match (
            self.row.is_public,
            self.row.start_shared,
            self.row.start_unverified,
        ) {
            (false, _, _) => StartPermission::Owner,
            (true, true, false) => StartPermission::Authenticated,
            (true, true, true) => StartPermission::Anonymous,
            (true, false, _) => StartPermission::Owner,
        }
    }

    pub fn wallets_id(&self) -> Vec<i64> {
        self.row
            .nodes
            .iter()
            .filter_map(|n| {
                (n.data.r#type == CommandType::Native && n.data.node_id == "wallet")
                    .then(|| {
                        n.data
                            .targets_form
                            .form_data
                            .get("wallet_id")
                            .and_then(|v| v.as_i64())
                    })
                    .flatten()
            })
            .collect()
    }

    pub fn interflows_id(&self) -> Vec<FlowId> {
        self.row
            .nodes
            .iter()
            .filter_map(|n| {
                let is_interflow = n.data.r#type == CommandType::Native
                    && (n.data.node_id == interflow::INTERFLOW
                        || n.data.node_id == interflow_instructions::INTERFLOW_INSTRUCTIONS);
                is_interflow
                    .then(|| interflow::get_interflow_id(&n.data).ok())
                    .flatten()
            })
            .collect()
    }
}

pub type DeploymentId = Uuid;

#[derive(bon::Builder, Debug, Clone)]
pub struct FlowDeployment {
    /// Deployment ID, NIL if not inserted yet
    pub id: DeploymentId,
    /// Owner of this deployment (and all flows belonging to it)
    pub user_id: UserId,
    /// Flow ID to start the set
    pub entrypoint: FlowId,
    /// Flow configs
    pub flows: HashMap<FlowId, Flow>,
    /// Wallets are stored separately
    pub wallets_id: Vec<i64>,

    /// Who can start the deployment
    pub start_permission: StartPermission,
    /// Stop flow and return transaction when available
    pub output_instructions: bool,
    /// Action's identity
    pub action_identity: Option<Pubkey>,
    /// List of public key and fee amount
    pub fees: Vec<(Pubkey, u64)>,
    /// Solana cluster and RPC URL
    pub solana_network: SolanaClientConfig,
}

impl FlowDeployment {
    fn new(entry: FlowRow) -> Self {
        let flow = Flow::builder().row(entry).build();
        Self::builder()
            .id(DeploymentId::nil())
            .entrypoint(flow.row.id)
            .start_permission(flow.start_permission())
            .wallets_id(flow.wallets_id())
            .user_id(flow.row.user_id)
            .solana_network(SolanaClientConfig {
                url: flow.row.current_network.url.clone(),
                cluster: flow.row.current_network.cluster,
            })
            .flows([(flow.row.id, flow)].into())
            .output_instructions(false)
            .fees(Vec::new())
            .build()
    }

    pub async fn from_entrypoint<S>(flow_id: FlowId, get_flow_row: &mut S) -> Result<Self, S::Error>
    where
        S: tower::Service<get_flow_row::Request, Response = get_flow_row::Response>,
    {
        let resp = get_flow_row
            .ready()
            .await?
            .call(get_flow_row::Request { flow_id })
            .await?;
        let mut dep = FlowDeployment::new(resp.row);

        let mut queue: Vec<FlowId> = dep
            .flows
            .values()
            .flat_map(|flow| flow.interflows_id())
            .collect();

        while let Some(id) = queue.pop() {
            if dep.flows.contains_key(&id) {
                continue;
            }

            let row = get_flow_row
                .ready()
                .await?
                .call(get_flow_row::Request { flow_id: id })
                .await?
                .row;
            let flow = Flow::builder().row(row).build();
            queue.extend(flow.interflows_id());
            dep.flows.insert(id, flow);
        }

        let mut wallets_id = dep
            .flows
            .values()
            .flat_map(|f| f.wallets_id())
            .collect::<Vec<_>>();
        wallets_id.sort_unstable();
        wallets_id.dedup();
        dep.wallets_id = wallets_id.clone();

        Ok(dep)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FlowStarter {
    pub user_id: UserId,
    pub pubkey: Pubkey,
    pub authenticated: bool,
    pub action_signer: Option<Pubkey>,
}

/// Start a flow deployment by starting the entrypoint
#[derive(Debug)]
pub struct StartFlowDeploymentOptions {
    pub inputs: ValueSet,
    pub starter: FlowStarter,
}

pub enum StartFlowOrigin {
    Start {},
    Interflow {
        flow_run_id: FlowRunId,
        node_id: NodeId,
        times: u32,
        depth: u32,
    },
}

#[derive(bon::Builder)]
pub struct FlowSet {
    deployment: FlowDeployment,
    context: FlowSetContext,
}

pub mod get_flow_row {
    use flow_lib::{config::client::FlowRow, utils::TowerClient, BoxError, FlowId};
    use thiserror::Error as ThisError;

    pub type Svc = TowerClient<Request, Response, Error>;

    pub struct Request {
        pub flow_id: FlowId,
    }

    impl actix::Message for Request {
        type Result = Result<Response, Error>;
    }

    pub struct Response {
        pub row: FlowRow,
    }

    #[derive(ThisError, Debug)]
    pub enum Error {
        #[error("flow not found")]
        NotFound,
        #[error("unauthorized")]
        Unauthorized,
        #[error(transparent)]
        Worker(tower::BoxError),
        #[error(transparent)]
        MailBox(#[from] actix::MailboxError),
        #[error(transparent)]
        Other(#[from] BoxError),
    }
}

pub mod make_signer {
    use flow_lib::{context::signer, utils::TowerClient, BoxError};
    use thiserror::Error as ThisError;

    pub type Svc = TowerClient<Request, Response, Error>;

    pub struct Request {
        pub wallets_id: Vec<i64>,
    }

    impl actix::Message for Request {
        type Result = Result<Response, Error>;
    }

    pub struct Response {
        pub signer: signer::Svc,
    }

    #[derive(ThisError, Debug)]
    pub enum Error {
        #[error(transparent)]
        Worker(tower::BoxError),
        #[error(transparent)]
        MailBox(#[from] actix::MailboxError),
        #[error(transparent)]
        Other(#[from] BoxError),
    }
}

fn to_client_config(flow: Flow) -> ClientConfig {
    ClientConfig {
        user_id: flow.row.user_id,
        id: flow.row.id,
        nodes: flow.row.nodes,
        edges: flow.row.edges,
        environment: flow.row.environment,
        sol_network: flow.row.current_network,
        instructions_bundling: flow.row.instructions_bundling,
        partial_config: None,
        collect_instructions: false,
        call_depth: 0,
        origin: FlowRunOrigin::Start {},
        signers: JsonValue::Null,
        interflow_instruction_info: Err("unimplemented".to_owned()),
    }
}

impl FlowSet {
    pub async fn start(
        self,
        options: StartFlowDeploymentOptions,
    ) -> Result<(FlowRunId, JoinHandle<FlowRunResult>), new_flow_run::Error> {
        let flow_id = self.deployment.entrypoint;
        let flow = self.deployment.flows.get(&flow_id).unwrap().clone();
        let shared_with = if flow.row.user_id != options.starter.user_id {
            [options.starter.user_id].into()
        } else {
            Vec::new()
        };
        let registry = FlowRegistry::builder()
            .flows(Arc::new(
                self.deployment
                    .flows
                    .into_iter()
                    .map(|(k, v)| (k, to_client_config(v)))
                    .collect(),
            ))
            .flow_owner(User {
                id: self.deployment.user_id,
            })
            .started_by(User {
                id: self.deployment.user_id,
            })
            .shared_with(shared_with)
            .signers_info(JsonValue::Null)
            .endpoints(self.context.endpoints)
            .depth(self.context.depth)
            .signer(self.context.signer)
            .token(self.context.get_jwt)
            .new_flow_run(self.context.new_flow_run)
            .get_previous_values(get_previous_values::unimplemented_svc())
            .rhai_permit(self.context.rhai_permit)
            .rhai_tx(self.context.rhai_tx)
            .maybe_parent_flow_execute(self.context.parent_flow_execute)
            .maybe_rpc_server(self.context.rpc_server)
            .build();
        let action_config = if let (Some(action_identity), Some(action_signer)) = (
            self.deployment.action_identity,
            options.starter.action_signer,
        ) {
            Some(SolanaActionConfig {
                action_signer,
                action_identity,
            })
        } else {
            None
        };
        registry
            .start(
                flow_id,
                options.inputs,
                StartFlowOptions {
                    partial_config: None,
                    collect_instructions: self.deployment.output_instructions,
                    action_identity: self.deployment.action_identity,
                    action_config,
                    fees: self.deployment.fees,
                    origin: FlowRunOrigin::Start {},
                    solana_client: Some(self.deployment.solana_network),
                    parent_flow_execute: None,
                    deployment_id: Some(self.deployment.id),
                },
            )
            .await
    }
}

#[derive(bon::Builder, Getters, Clone)]
pub struct FlowSetContext {
    depth: u32,
    endpoints: Endpoints,

    signer: signer::Svc,
    get_jwt: get_jwt::Svc,
    new_flow_run: new_flow_run::Svc,
    parent_flow_execute: Option<execute::Svc>,

    #[builder(default = Arc::new(Semaphore::new(1)))]
    rhai_permit: Arc<Semaphore>,
    #[builder(default)]
    rhai_tx: Arc<Mutex<Option<crossbeam_channel::Sender<run_rhai::ChannelMessage>>>>,

    rpc_server: Option<actix::Addr<srpc::Server>>,
}

/*
pub struct FlowContext {
    set_context: FlowSetContext,
    flow_run_id: FlowRunId,
    http: reqwest::Client,
    solana_rpc: Arc<SolanaClient>,
    parent_flow_execute: execute::Svc,
}

pub struct Context {
    flow: FlowContext,
    execute: execute::Svc,
    start_interflow: start_interflow::Svc,
    node_id: NodeId,
    times: u32,
}
*/
