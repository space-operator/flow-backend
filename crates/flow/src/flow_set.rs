use flow_lib::{
    config::{
        client::{BundlingMode, ClientConfig, FlowRow, FlowRunOrigin, Network, PartialConfig},
        Endpoints,
    },
    context::{execute, get_jwt, signer},
    solana::{ExecuteOn, ExecutionConfig, Pubkey, SolanaActionConfig},
    CommandType, FlowId, FlowRunId, NodeId, SolanaClientConfig, User, UserId, ValueSet,
};
use getset::Getters;
use hashbrown::HashMap;
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};
use tokio::{sync::Semaphore, task::JoinHandle};

use crate::{
    command::{interflow, interflow_instructions},
    flow_graph::FlowRunResult,
    flow_registry::{get_previous_values, new_flow_run, run_rhai, FlowRegistry, StartFlowOptions},
};

/// Who can start flows
pub enum StartPermission {
    /// Only flow owner can start
    Owner,
    /// Any authenticated user
    Authenticated,
    /// Any unauthenticated user
    Anonymous,
}

#[derive(bon::Builder, Clone)]
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

#[derive(bon::Builder)]
pub struct FlowDeployment {
    /// Owner of this deployment (and all flows belonging to it)
    pub user_id: UserId,
    /// Flow ID to start the set
    pub entrypoint: FlowId,
    /// Flow configs
    pub flows: HashMap<FlowId, Flow>,

    /// Who can start the deployment
    pub start_permission: StartPermission,
    /// Wallets are stored separately
    pub wallets_id: Vec<i64>,

    pub collect_instructions: bool,
    pub action_identity: Option<Pubkey>,
    pub action_config: Option<SolanaActionConfig>,
    pub fees: Vec<(Pubkey, u64)>,
}

impl FlowDeployment {
    pub fn new(entry: FlowRow) -> Self {
        let flow = Flow::builder().row(entry).build();
        Self::builder()
            .entrypoint(flow.row.id)
            .start_permission(flow.start_permission())
            .wallets_id(flow.wallets_id())
            .user_id(flow.row.user_id)
            .flows([(flow.row.id, flow)].into())
            .collect_instructions(false)
            .fees(Vec::new())
            .build()
    }
}

pub struct FlowStarter {
    pub user_id: UserId,
    pub pubkey: Pubkey,
    pub authenticated: bool,
}

/// Start a flow deployment by starting the entrypoint
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

pub struct FlowSet {
    flows: FlowDeployment,

    ctx: FlowSetContext,
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
    pub async fn from_entrypoint(
        flow_id: FlowId,
        mut get_flow_row_svc: get_flow_row::Svc,
        ctx: FlowSetContext,
    ) -> crate::Result<FlowSet> {
        let resp = get_flow_row_svc
            .call_ref(get_flow_row::Request { flow_id })
            .await?;
        let mut dep = FlowDeployment::new(resp.row);

        let mut queue: Vec<FlowId> = dep
            .flows
            .values()
            .map(|flow| flow.interflows_id())
            .flatten()
            .collect();

        while let Some(id) = queue.pop() {
            if dep.flows.contains_key(&id) {
                continue;
            }

            let row = get_flow_row_svc
                .call_mut(get_flow_row::Request { flow_id: id })
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

        Ok(FlowSet { flows: dep, ctx })
    }

    pub async fn start(
        self,
        options: StartFlowDeploymentOptions,
    ) -> Result<(FlowRunId, JoinHandle<FlowRunResult>), new_flow_run::Error> {
        let flow_id = self.flows.entrypoint;
        let flow = self.flows.flows.get(&flow_id).unwrap().clone();
        let registry = FlowRegistry::builder()
            .flows(Arc::new(
                self.flows
                    .flows
                    .into_iter()
                    .map(|(k, v)| (k, to_client_config(v)))
                    .collect(),
            ))
            .flow_owner(User {
                id: self.flows.user_id,
            })
            .started_by(User {
                id: self.flows.user_id,
            })
            .shared_with(Vec::new())
            .signers_info(JsonValue::Null)
            .endpoints(self.ctx.endpoints)
            .depth(self.ctx.depth)
            .signer(self.ctx.signer)
            .token(self.ctx.get_jwt)
            .new_flow_run(self.ctx.new_flow_run)
            .get_previous_values(get_previous_values::unimplemented_svc())
            .rhai_permit(self.ctx.rhai_permit)
            .rhai_tx(self.ctx.rhai_tx)
            .maybe_parent_flow_execute(self.ctx.parent_flow_execute)
            .maybe_rpc_server(self.ctx.rpc_server)
            .build();
        registry
            .start(
                flow_id,
                options.inputs,
                StartFlowOptions {
                    partial_config: None,
                    collect_instructions: self.flows.collect_instructions,
                    action_identity: self.flows.action_identity,
                    action_config: self.flows.action_config,
                    fees: self.flows.fees,
                    origin: FlowRunOrigin::Start {},
                    solana_client: Some(SolanaClientConfig {
                        url: flow.row.current_network.url.clone(),
                        cluster: flow.row.current_network.cluster,
                    }),
                    parent_flow_execute: None,
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

    rhai_permit: Arc<Semaphore>,
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
