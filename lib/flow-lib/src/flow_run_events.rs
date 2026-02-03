use crate::{NodeId, context::signer::SignatureRequest};
use bincode::{Decode, Encode};
use chrono::{DateTime, Utc};
use futures::channel::mpsc;
use serde::Serialize;
use value::Value;

#[derive(derive_more::From, actix::Message, Clone, Debug, Serialize)]
#[rtype(result = "()")]
#[serde(tag = "event", content = "data")]
pub enum Event {
    FlowStart(FlowStart),
    FlowError(FlowError),
    FlowLog(FlowLog),
    FlowFinish(FlowFinish),
    NodeStart(NodeStart),
    NodeOutput(NodeOutput),
    NodeError(NodeError),
    NodeLog(NodeLog),
    NodeFinish(NodeFinish),
    SignatureRequest(SignatureRequest),
    ApiInput(ApiInput),
}

impl Event {
    pub fn time(&self) -> DateTime<Utc> {
        match self {
            Event::FlowStart(e) => e.time,
            Event::FlowError(e) => e.time,
            Event::FlowLog(e) => e.time,
            Event::FlowFinish(e) => e.time,
            Event::NodeStart(e) => e.time,
            Event::NodeOutput(e) => e.time,
            Event::NodeError(e) => e.time,
            Event::NodeLog(e) => e.time,
            Event::NodeFinish(e) => e.time,
            Event::SignatureRequest(e) => e.time,
            Event::ApiInput(e) => e.time,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Default, Encode, Decode)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.serialize(f)
    }
}

impl From<tracing::Level> for LogLevel {
    fn from(value: tracing::Level) -> Self {
        match value {
            tracing::Level::TRACE => LogLevel::Trace,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
        }
    }
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct ApiInput {
    pub time: DateTime<Utc>,
    pub url: String,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct FlowStart {
    pub time: DateTime<Utc>,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct FlowError {
    pub time: DateTime<Utc>,
    pub error: String,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct FlowLog {
    pub time: DateTime<Utc>,
    pub level: LogLevel,
    pub module: Option<String>,
    pub content: String,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct FlowFinish {
    pub time: DateTime<Utc>,
    pub not_run: Vec<NodeId>,
    pub output: Value,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct NodeStart {
    pub time: DateTime<Utc>,
    pub node_id: NodeId,
    pub times: u32,
    pub input: Value,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct NodeOutput {
    pub time: DateTime<Utc>,
    pub node_id: NodeId,
    pub times: u32,
    pub output: Value,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct NodeError {
    pub time: DateTime<Utc>,
    pub node_id: NodeId,
    pub times: u32,
    pub error: String,
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct NodeLog {
    pub time: DateTime<Utc>,
    pub node_id: NodeId,
    pub times: u32,
    pub level: LogLevel,
    pub module: Option<String>,
    pub content: String,
}

#[derive(Encode, Decode, Debug)]
pub struct NodeLogContent {
    #[bincode(with_serde)]
    pub time: DateTime<Utc>,
    pub level: LogLevel,
    pub module: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct NodeLogSender {
    node_id: NodeId,
    times: u32,
    tx: EventSender,
}

impl NodeLogSender {
    pub fn new(tx: EventSender, node_id: NodeId, times: u32) -> Self {
        Self { node_id, times, tx }
    }

    pub fn send(
        &self,
        NodeLogContent {
            time,
            level,
            module,
            content,
        }: NodeLogContent,
    ) -> Result<(), mpsc::SendError> {
        self.tx
            .unbounded_send(Event::NodeLog(NodeLog {
                time,
                node_id: self.node_id,
                times: self.times,
                level,
                module,
                content,
            }))
            .map_err(|error| error.into_send_error())
    }
}

#[derive(actix::Message, Default, Clone, Debug, Serialize)]
#[rtype(result = "()")]
pub struct NodeFinish {
    pub time: DateTime<Utc>,
    pub node_id: NodeId,
    pub times: u32,
}

pub fn channel() -> (EventSender, EventReceiver) {
    futures::channel::mpsc::unbounded()
}
pub type EventSender = futures::channel::mpsc::UnboundedSender<Event>;
pub type EventReceiver = futures::channel::mpsc::UnboundedReceiver<Event>;

pub fn event_channel() -> (EventSender, EventReceiver) {
    futures::channel::mpsc::unbounded()
}

pub const DEFAULT_LOG_FILTER: &str = "info,solana_client=debug";
pub const FLOW_SPAN_NAME: &str = "flow_logs";
pub const NODE_SPAN_NAME: &str = "node_logs";
