use flow_lib::{
    command::InstructionInfo,
    config::client::{BundlingMode, Edge, Network, Node},
    FlowId, UserId,
};
use hashbrown::HashMap;

/// Who can start flows
pub enum StartPermission {
    /// Only flow owner can start
    Owner,
    /// Any authenticated user
    Authenticated,
    /// Any unauthenticated user
    Anonymous,
}

pub struct Resources {}

pub struct ParsedFlow {
    pub id: FlowId,
    pub user_id: UserId,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub environment: HashMap<String, String>,
    pub current_network: Network,
    pub instructions_bundling: BundlingMode,
    pub is_public: bool,
    pub start_shared: bool,
    pub start_unverified: bool,
    pub interflow_instruction_info: Result<InstructionInfo, String>,
}

pub struct FlowDeployment {
    /// Flow configs
    pub flows: HashMap<FlowId, ParsedFlow>,
    /// Flow ID to call
    pub entrypoint: FlowId,
    /// Who can start the deployment
    pub start_permission: StartPermission,
    /// Environment customization
    pub environment: Option<HashMap<String, String>>,
    /// Solana network customization
    pub sol_network: Option<Network>,
    /// Bundling mode customization
    pub bundling_mode: Option<BundlingMode>,
}
