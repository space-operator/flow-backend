use flow_lib::{config::client::ClientConfig, FlowId, FlowRunId, ValueSet};
use thiserror::Error as ThisError;
use uuid::Uuid;

pub struct GetFlowConfig {
    pub user_id: Uuid,
    pub flow_id: FlowId,
}

impl actix::Message for GetFlowConfig {
    type Result = Result<ClientConfig, anyhow::Error>;
}

pub struct StartFlow {
    pub user_id: Uuid,
    pub flow_id: FlowId,
    pub input: ValueSet,
}

impl actix::Message for StartFlow {
    type Result = Result<FlowRunId, anyhow::Error>;
}

pub type SubscriptionID = u64;

#[derive(ThisError, Debug)]
pub enum SubscribeError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error(transparent)]
    MailBox(#[from] actix::MailboxError),
}

pub struct Finished {
    pub sub_id: SubscriptionID,
}

impl actix::Message for Finished {
    type Result = ();
}
