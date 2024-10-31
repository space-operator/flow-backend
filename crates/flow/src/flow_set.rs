use flow_lib::{
    config::client::{BundlingMode, ClientConfig, Network},
    FlowId,
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

pub struct FlowDeployment {
    pub flows: HashMap<FlowId, ClientConfig>,
    pub entrypoint: FlowId,
    pub environment: HashMap<String, String>,
    pub sol_network: Network,
    pub bundling_mode: BundlingMode,
    pub start_permission: StartPermission,
}
