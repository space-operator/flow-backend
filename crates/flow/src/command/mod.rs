pub mod prelude {
    pub use async_trait::async_trait;
    pub use flow_lib::{
        CmdInputDescription as Input, CmdOutputDescription as Output, FlowId, Name, Value,
        ValueSet, ValueType,
        command::{
            CommandDescription, CommandError, CommandTrait,
            builder::{BuildResult, BuilderCache, CmdBuilder},
        },
        config::client::NodeData,
        context::CommandContext,
    };
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::Value as JsonValue;
    pub use std::sync::Arc;
    pub use thiserror::Error as ThisError;
}

pub mod api_input;
pub mod collect;
pub mod flow_input;
pub mod flow_output;
pub mod foreach;
pub mod interflow;
pub mod interflow_instructions;
pub mod rhai;
pub mod wallet;
// pub mod wasm;
