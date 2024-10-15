//! Note: only add fields that are needed in backend.
use crate::command::InstructionInfo;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Definition {
    pub r#type: super::CommandType,
    pub data: Data,
    pub sources: Vec<Source>,
    pub targets: Vec<Target>,
    #[serde(default)]
    pub permissions: Permissions,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Permissions {
    #[serde(default)]
    pub user_tokens: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Data {
    pub node_id: String,
    pub instruction_info: Option<InstructionInfo>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Source {
    pub name: String,
    pub r#type: super::ValueType,
    #[serde(default = "value::default::bool_false")]
    pub optional: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Target {
    pub name: String,
    pub type_bounds: Vec<super::ValueType>,
    pub required: bool,
    pub passthrough: bool,
}
