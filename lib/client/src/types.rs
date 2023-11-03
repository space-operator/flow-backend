use chrono::{DateTime, Utc};
use flow_lib::{
    context::signer::SignatureRequest,
    solana::{Pubkey, Signature},
    value::Value,
    FlowRunId, NodeId, UserId,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum RestResult<T> {
    Error(ErrorBody),
    Success(T),
}

#[derive(Debug)]
pub struct True;

impl Serialize for True {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        true.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for True {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = bool::deserialize(deserializer)?;
        if value {
            Ok(Self)
        } else {
            Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Bool(value),
                &"true",
            ))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorBody {
    pub error: String,
}

pub mod start_flow {
    use super::*;
    use flow_lib::config::client::PartialConfig;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Params {
        #[serde(default)]
        pub inputs: HashMap<String, Value>,
        pub partial_config: Option<PartialConfig>,
        #[serde(default)]
        pub environment: HashMap<String, String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Output {
        pub flow_run_id: FlowRunId,
    }
}

pub mod start_flow_unverified {
    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Params {
        #[serde(default)]
        pub inputs: HashMap<String, Value>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Output {
        pub flow_run_id: FlowRunId,
        pub token: String,
    }
}

pub mod start_flow_shared {
    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Params {
        #[serde(default)]
        pub inputs: HashMap<String, Value>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Output {
        pub flow_run_id: FlowRunId,
    }
}

pub mod stop_flow {
    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Params {
        #[serde(default)]
        pub timeout_millies: u32,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Output {
        pub success: True,
    }
}

pub mod submit_signature {
    use super::*;

    #[serde_as]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct Params {
        id: i64,
        #[serde_as(as = "DisplayFromStr")]
        signature: Signature,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Output {
        pub success: True,
    }
}

pub mod ws {
    use super::*;
    use std::borrow::Cow;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct WsRequest<T> {
        id: i64,
        method: Cow<'static, str>,
        params: T,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct WsReply<T> {
        id: i64,
        #[serde(flatten)]
        result: Result<T, String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct WsEvent<T> {
        stream_id: i64,
        #[serde(flatten)]
        event: T,
    }

    pub mod authenticate {
        use super::*;

        pub const METHOD: Cow<'static, str> = Cow::Borrowed("Authenticate");

        #[derive(Serialize, Deserialize, Debug)]
        pub struct Params {
            pub token: String,
        }

        #[serde_as]
        #[derive(Serialize, Deserialize, Debug)]

        pub struct Output {
            pub user_id: Option<UserId>,
            #[serde_as(as = "Option<DisplayFromStr>")]
            pub pubkey: Option<Pubkey>,
            pub flow_run_id: Option<FlowRunId>,
        }
    }

    pub mod subscribe_flow_run_events {
        use super::*;

        pub const METHOD: Cow<'static, str> = Cow::Borrowed("SubscribeFlowRunEvents");

        #[derive(Serialize, Deserialize, Debug)]
        pub struct Params {
            pub flow_run_id: FlowRunId,
            pub token: Option<String>,
        }

        #[derive(Serialize, Deserialize, Debug)]

        pub struct Output {
            pub stream_id: i64,
        }

        #[derive(Serialize, Deserialize, Debug)]
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
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub enum LogLevel {
            Trace,
            Debug,
            Info,
            Warn,
            Error,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct FlowStart {
            pub flow_run_id: FlowRunId,
            pub time: DateTime<Utc>,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct FlowError {
            pub flow_run_id: FlowRunId,
            pub time: DateTime<Utc>,
            pub error: String,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct FlowLog {
            pub flow_run_id: FlowRunId,
            pub time: DateTime<Utc>,
            pub level: LogLevel,
            pub module: Option<String>,
            pub content: String,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct FlowFinish {
            pub flow_run_id: FlowRunId,
            pub time: DateTime<Utc>,
            pub not_run: Vec<NodeId>,
            pub output: Value,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct NodeStart {
            pub flow_run_id: FlowRunId,
            pub time: DateTime<Utc>,
            pub node_id: NodeId,
            pub times: u32,
            pub input: Value,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct NodeOutput {
            pub flow_run_id: FlowRunId,
            pub time: DateTime<Utc>,
            pub node_id: NodeId,
            pub times: u32,
            pub output: Value,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct NodeError {
            pub flow_run_id: FlowRunId,
            pub time: DateTime<Utc>,
            pub node_id: NodeId,
            pub times: u32,
            pub error: String,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct NodeLog {
            pub flow_run_id: FlowRunId,
            pub time: DateTime<Utc>,
            pub node_id: NodeId,
            pub times: u32,
            pub level: LogLevel,
            pub module: Option<String>,
            pub content: String,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct NodeFinish {
            pub flow_run_id: FlowRunId,
            pub time: DateTime<Utc>,
            pub node_id: NodeId,
            pub times: u32,
        }
    }

    pub mod subscribe_signature_requests {
        use super::*;

        pub const METHOD: Cow<'static, str> = Cow::Borrowed("SubscribeSignatureRequests");

        #[derive(Serialize, Deserialize, Debug)]
        pub struct Params {}

        #[derive(Serialize, Deserialize, Debug)]

        pub struct Output {
            pub stream_id: i64,
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(tag = "event", content = "data")]

        pub enum Event {
            SignatureRequest(SignatureRequest),
        }
    }
}
