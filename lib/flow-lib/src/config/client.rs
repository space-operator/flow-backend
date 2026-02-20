//! Parse JS front-end flow config into back-end flow config

use crate::{
    CmdInputDescription, CmdOutputDescription, CommandType, FlowId, FlowRunId, NodeId,
    SolanaClientConfig, SolanaNet, UserId, ValueType, command::InstructionInfo,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue, value::RawValue};
use serde_with::serde_as;
use std::collections::HashMap;
use uuid::Uuid;
use value::default::bool_false;

fn default_interflow_instruction_info() -> Result<InstructionInfo, String> {
    Err("not available".to_string())
}

/// A row of `flows_v2` table and `flow_deployments_flows.data` column
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowRow {
    pub id: FlowId,
    pub user_id: UserId,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    // TODO: remove default
    #[serde(default)]
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull")]
    pub environment: HashMap<String, String>,
    pub current_network: Network,
    pub instructions_bundling: BundlingMode,
    pub is_public: bool,
    pub start_shared: bool,
    pub start_unverified: bool,
}

impl FlowRow {
    /// Serialize data for `flow_deployments_flows.data` column.
    pub fn data(&self) -> Result<Box<RawValue>, serde_json::Error> {
        serde_json::value::to_raw_value(self)
    }
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
            url: SolanaNet::Devnet.url(),
            cluster: SolanaNet::Devnet,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub data: NodeData,
}

fn default_native_str() -> String {
    "native".to_owned()
}

fn default_free_type() -> ValueType {
    ValueType::Free
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Ports {
    #[serde(default)]
    pub inputs: Vec<InputPort>,
    #[serde(default)]
    pub outputs: Vec<OutputPort>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputPort {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub type_bounds: Vec<ValueType>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub passthrough: bool,
    #[serde(default)]
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputPort {
    pub id: Uuid,
    pub name: String,
    #[serde(default = "default_free_type")]
    pub r#type: ValueType,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeV2 {
    pub id: NodeId,
    #[serde(default = "default_native_str")]
    pub r#type: String,
    #[serde(default)]
    pub position: Option<Position>,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    pub data: NodeDataV2,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeDataV2 {
    pub node_id: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub ports: Ports,
    #[serde(default)]
    pub config: HashMap<String, JsonValue>,
    #[serde(default)]
    pub style: Option<JsonValue>,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientConfigV2 {
    pub user_id: UserId,
    pub id: FlowId,
    pub nodes: Vec<NodeV2>,
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
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlowRowV2 {
    pub id: FlowId,
    pub user_id: UserId,
    pub nodes: Vec<NodeV2>,
    pub edges: Vec<Edge>,
    #[serde(default)]
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull")]
    pub environment: HashMap<String, String>,
    pub current_network: Network,
    #[serde(default)]
    pub instructions_bundling: BundlingMode,
    pub is_public: bool,
    pub start_shared: bool,
    pub start_unverified: bool,
    #[serde(default)]
    pub current_branch_id: Option<i32>,
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

impl From<InputPort> for Target {
    fn from(p: InputPort) -> Self {
        Self {
            id: p.id,
            name: p.name,
            type_bounds: p.type_bounds,
            required: p.required,
            passthrough: p.passthrough,
        }
    }
}

impl From<OutputPort> for Source {
    fn from(p: OutputPort) -> Self {
        Self {
            id: p.id,
            name: p.name,
            r#type: p.r#type,
            optional: p.optional,
        }
    }
}

fn infer_command_type(node_id: &str) -> CommandType {
    let name = node_id.rsplit('/').next().unwrap_or(node_id);
    if name == "deno_script" {
        CommandType::Deno
    } else {
        CommandType::Native
    }
}

impl From<NodeDataV2> for NodeData {
    fn from(v2: NodeDataV2) -> Self {
        let form_data = JsonValue::Object(v2.config.into_iter().collect::<JsonMap<_, _>>());
        Self {
            r#type: infer_command_type(&v2.node_id),
            node_id: v2.node_id,
            sources: v2.ports.outputs.into_iter().map(Into::into).collect(),
            targets: v2.ports.inputs.into_iter().map(Into::into).collect(),
            targets_form: TargetsForm {
                form_data,
                extra: Extra::default(),
                wasm_bytes: None,
            },
            instruction_info: None,
        }
    }
}

impl From<NodeV2> for Node {
    fn from(v2: NodeV2) -> Self {
        Self {
            id: v2.id,
            data: v2.data.into(),
        }
    }
}

impl From<ClientConfigV2> for ClientConfig {
    fn from(v2: ClientConfigV2) -> Self {
        Self {
            user_id: v2.user_id,
            id: v2.id,
            nodes: v2.nodes.into_iter().map(Into::into).collect(),
            edges: v2.edges,
            environment: v2.environment,
            sol_network: v2.sol_network,
            instructions_bundling: v2.instructions_bundling,
            partial_config: v2.partial_config,
            collect_instructions: v2.collect_instructions,
            call_depth: v2.call_depth,
            origin: v2.origin,
            signers: v2.signers,
            interflow_instruction_info: default_interflow_instruction_info(),
        }
    }
}

impl From<FlowRowV2> for FlowRow {
    fn from(v2: FlowRowV2) -> Self {
        Self {
            id: v2.id,
            user_id: v2.user_id,
            nodes: v2.nodes.into_iter().map(Into::into).collect(),
            edges: v2.edges,
            environment: v2.environment,
            current_network: v2.current_network,
            instructions_bundling: v2.instructions_bundling,
            is_public: v2.is_public,
            start_shared: v2.start_shared,
            start_unverified: v2.start_unverified,
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

impl Default for TargetsForm {
    fn default() -> Self {
        Self {
            form_data: JsonValue::Object(<_>::default()),
            extra: Extra::default(),
            wasm_bytes: None,
        }
    }
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
