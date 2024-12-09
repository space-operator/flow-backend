use std::sync::{Arc, Mutex};

use flow_lib::{
    config::client::{BundlingMode, FlowRow, Network, PartialConfig},
    solana::{ExecuteOn, Pubkey},
    FlowId, FlowRunId, NodeId, UserId, ValueSet,
};
use hashbrown::HashMap;
use tokio::sync::Semaphore;

/// Who can start flows
pub enum StartPermission {
    /// Only flow owner can start
    Owner,
    /// Any authenticated user
    Authenticated,
    /// Any unauthenticated user
    Anonymous,
}

pub struct Flow {
    pub row: FlowRow,

    pub partial_config: Option<PartialConfig>,
    pub output_instructions: bool,
    pub bundling_mode: Option<BundlingMode>,
    pub inherit_bundling_mode: Option<bool>,
}

pub struct FlowSet {
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
    // start_interflow: start_interflow::Svc,
    node_id: NodeId,
    times: u32,
}
