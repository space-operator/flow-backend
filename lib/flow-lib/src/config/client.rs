//! Parse JS front-end flow config into back-end flow config

use crate::{
    command::InstructionInfo, CmdInputDescription, CmdOutputDescription, CommandType, FlowId,
    FlowRunId, NodeId, SolanaClientConfig, SolanaNet, UserId, ValueType,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_with::serde_as;
use std::collections::HashMap;
use uuid::Uuid;
use value::default::bool_false;

fn default_interflow_instruction_info() -> Result<InstructionInfo, String> {
    Err("not available".to_string())
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientConfig {
    pub user_id: UserId,
    pub id: FlowId,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    #[serde(default)]
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull")]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull")]
    pub sol_network: Network,
    #[serde(default)]
    pub instructions_bundling: BundlingMode,
    #[serde(default)]
    pub partial_config: Option<PartialConfig>,
    #[serde(default)]
    pub collect_instructions: bool,
    #[serde(default)]
    pub call_depth: u32,
    #[serde(default = "default_origin")]
    pub origin: FlowRunOrigin,
    #[serde(default)]
    pub signers: JsonValue,
    #[serde(default = "default_interflow_instruction_info")]
    pub interflow_instruction_info: Result<InstructionInfo, String>,
}

const fn default_origin() -> FlowRunOrigin {
    FlowRunOrigin::Start {}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowRunOrigin {
    Start {},
    StartShared {
        started_by: UserId,
    },
    Interflow {
        flow_run_id: FlowRunId,
        node_id: NodeId,
        times: u32,
    },
}

impl Default for FlowRunOrigin {
    fn default() -> Self {
        Self::Start {}
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValuesConfig {
    #[serde(default)]
    pub nodes: HashMap<NodeId, FlowRunId>,
    pub default_run_id: Option<FlowRunId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartialConfig {
    pub only_nodes: Vec<NodeId>,
    pub values_config: ValuesConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BundlingMode {
    #[default]
    Off,
    Automatic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Network {
    pub url: String,
    pub cluster: SolanaNet,
}

impl From<SolanaClientConfig> for Network {
    fn from(value: SolanaClientConfig) -> Self {
        Self {
            url: value.url,
            cluster: value.cluster,
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        Self {
            url: "https://norrie-yvr0sx-fast-devnet.helius-rpc.com".to_owned(),
            cluster: SolanaNet::Devnet,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
    pub data: NodeData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeData {
    pub r#type: CommandType,
    pub node_id: String,
    pub sources: Vec<Source>,
    pub targets: Vec<Target>,
    pub targets_form: TargetsForm,
    pub instruction_info: Option<InstructionInfo>,
}

impl NodeData {
    pub fn inputs(&self) -> Vec<CmdInputDescription> {
        self.targets.iter().cloned().map(Into::into).collect()
    }
    pub fn outputs(&self) -> Vec<CmdOutputDescription> {
        self.sources.iter().cloned().map(Into::into).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeDataSkipWasm {
    pub r#type: CommandType,
    pub node_id: String,
    pub sources: Vec<Source>,
    pub targets: Vec<Target>,
    pub targets_form: TargetsFormSkipWasm,
    pub instruction_info: Option<InstructionInfo>,
}

impl From<NodeData> for NodeDataSkipWasm {
    fn from(
        NodeData {
            r#type,
            node_id,
            sources,
            targets,
            targets_form,
            instruction_info,
        }: NodeData,
    ) -> Self {
        let TargetsForm {
            form_data, extra, ..
        } = targets_form;
        NodeDataSkipWasm {
            r#type,
            node_id,
            sources,
            targets,
            targets_form: TargetsFormSkipWasm { form_data, extra },
            instruction_info,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub id: Uuid,
    pub name: String,
    pub r#type: ValueType,
    #[serde(default = "bool_false")]
    pub optional: bool,
}

impl From<Source> for CmdOutputDescription {
    fn from(
        Source {
            name,
            r#type,
            optional,
            ..
        }: Source,
    ) -> Self {
        Self {
            name,
            r#type,
            optional,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub id: Uuid,
    pub name: String,
    pub type_bounds: Vec<ValueType>,
    pub required: bool,
    pub passthrough: bool,
}

impl From<Target> for CmdInputDescription {
    fn from(
        Target {
            name,
            type_bounds,
            required,
            passthrough,
            ..
        }: Target,
    ) -> Self {
        Self {
            name,
            type_bounds,
            required,
            passthrough,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetsForm {
    pub form_data: JsonValue,
    #[serde(default)]
    pub extra: Extra,

    pub wasm_bytes: Option<bytes::Bytes>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetsFormSkipWasm {
    pub form_data: JsonValue,
    #[serde(default)]
    pub extra: Extra,
}

impl std::fmt::Debug for TargetsForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TargetsForm")
            .field("form_data", &self.form_data)
            .field("extra", &self.extra)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for TargetsFormSkipWasm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TargetsForm")
            .field("form_data", &self.form_data)
            .field("extra", &self.extra)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Extra {
    // for WASM node
    pub supabase_id: Option<i64>,
    #[serde(flatten)]
    pub rest: HashMap<String, JsonValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceHandle {
    pub id: Uuid,
    pub is_passthough: bool,
}

impl Serialize for SourceHandle {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.is_passthough {
            format!("passthrough-{}", self.id).serialize(s)
        } else {
            self.id.serialize(s)
        }
    }
}

impl<'de> Deserialize<'de> for SourceHandle {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const PREFIX: &str = "passthrough-";
        let s = String::deserialize(d)?;
        let (is_passthough, uuid_str) = if s.starts_with(PREFIX) {
            (true, &s.as_str()[PREFIX.len()..])
        } else {
            (false, s.as_str())
        };

        Ok(Self {
            is_passthough,
            id: uuid_str.parse().map_err(serde::de::Error::custom)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub source: Uuid,
    #[serde(rename = "sourceHandle")]
    pub source_handle: SourceHandle,
    pub target: Uuid,
    #[serde(rename = "targetHandle")]
    pub target_handle: Uuid,
}
