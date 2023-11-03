pub mod collect;
pub mod flow_input;
pub mod flow_output;
pub mod foreach;
pub mod interflow;
pub mod interflow_instructions;
pub mod rhai;
pub mod wasm;

pub mod prelude {
    pub use async_trait::async_trait;
    pub use flow_lib::{
        command::{
            builder::{BuildResult, BuilderCache, CmdBuilder},
            CommandDescription, CommandError, CommandTrait,
        },
        config::client::NodeData,
        context::Context,
        CmdInputDescription as Input, CmdOutputDescription as Output, FlowId, Name, Value,
        ValueSet, ValueType,
    };
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::Value as JsonValue;
    pub use std::sync::Arc;
    pub use thiserror::Error as ThisError;
}
