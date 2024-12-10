use std::{collections::VecDeque, sync::Arc};

use anyhow::anyhow;
use flow_lib::{
    config::client::{BundlingMode, FlowRow, Network, PartialConfig},
    context::signer,
    solana::{ExecuteOn, Pubkey},
    CommandType, FlowId, FlowRunId, NodeId, UserId, ValueSet,
};
use hashbrown::HashMap;

use crate::command::{interflow, interflow_instructions};

/// Who can start flows
pub enum StartPermission {
    /// Only flow owner can start
    Owner,
    /// Any authenticated user
    Authenticated,
    /// Any unauthenticated user
    Anonymous,
}

#[derive(bon::Builder)]
pub struct Flow {
    pub row: FlowRow,

    pub partial_config: Option<PartialConfig>,
    pub output_instructions: bool,
    pub bundling_mode: Option<BundlingMode>,
    pub inherit_bundling_mode: Option<bool>,
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
    /// Flow ID to start the set
    pub entrypoint: FlowId,
    /// Flow configs
    pub flows: HashMap<FlowId, Flow>,

    /// Who can start the deployment
    pub start_permission: StartPermission,
    /// Wallets are stored separately
    pub wallets_id: Vec<i64>,
    /// Solana execution config
    pub execution_config: ExecuteOn,

    /// Environment variables customization
    pub environment: Option<HashMap<String, String>>,
    /// Solana network customization
    pub sol_network: Option<Network>,
    /// Addresses and amounts to send fees to
    pub fees: Option<Vec<(Pubkey, u64)>>,
}

impl FlowDeployment {
    pub fn new(entry: FlowRow) -> Self {
        let flow = Flow::builder()
            .row(entry)
            .output_instructions(false)
            .build();
        Self::builder()
            .entrypoint(flow.row.id)
            .start_permission(flow.start_permission())
            .wallets_id(flow.wallets_id())
            .execution_config(ExecuteOn::CurrentMachine)
            .flows([(flow.row.id, flow)].into())
            .build()
    }
}

pub struct FlowStarter {
    pub user_id: UserId,
    pub pubkey: Pubkey,
    pub authenticated: bool,
}

pub struct StartFlowOptions {
    pub flow_id: FlowId,
    pub inputs: ValueSet,
    pub origin: StartFlowOrigin,
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
    flows: Arc<FlowDeployment>,

    signers: signer::Svc,
}

pub mod get_flow_row {
    use flow_lib::{config::client::FlowRow, utils::TowerClient, BoxError, FlowId, UserId};
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

impl FlowSet {
    pub async fn from_entrypoint(
        flow_id: FlowId,
        get_flow_row_svc: get_flow_row::Svc,
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
                .call_ref(get_flow_row::Request { flow_id: id })
                .await?
                .row;
            let flow = Flow::builder().row(row).output_instructions(false).build();
            queue.extend(flow.interflows_id());
            dep.flows.insert(id, flow);
        }

        Ok(FlowSet {
            flows: Arc::new(dep),
            signers: signer::unimplemented_svc(),
        })
    }
}

/*
pub struct FlowSetContext {
    endpoints: Endpoints,

    signer: signer::Svc,
    get_jwt: get_jwt::Svc,
    new_flow_run: new_flow_run::Svc,

    rhai_permit: Arc<Semaphore>,
    rhai_tx: Arc<Mutex<Option<crossbeam_channel::Sender<run_rhai::ChannelMessage>>>>,

    rpc_server: Option<actix::Addr<srpc::Server>>,
}

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
