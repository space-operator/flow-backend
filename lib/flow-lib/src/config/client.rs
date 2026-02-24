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
    #[serde(default)]
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull")]
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
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull")]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
}

impl From<OutputPort> for CmdOutputDescription {
    fn from(p: OutputPort) -> Self {
        Self {
            name: p.name,
            r#type: p.r#type,
            optional: p.optional,
        }
    }
}

impl From<InputPort> for CmdInputDescription {
    fn from(p: InputPort) -> Self {
        Self {
            name: p.name,
            type_bounds: p.type_bounds,
            required: p.required,
            passthrough: p.passthrough,
        }
    }
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
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull")]
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
    #[serde_as(deserialize_as = "serde_with::DefaultOnNull")]
    pub instructions_bundling: BundlingMode,
    pub is_public: bool,
    pub start_shared: bool,
    pub start_unverified: bool,
    #[serde(default)]
    pub current_branch_id: Option<i32>,
}

/// Runtime state for WASM nodes: DB row ID + cached bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmNode {
    pub supabase_id: i64,
    pub bytes: Option<bytes::Bytes>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeData {
    pub r#type: CommandType,
    pub node_id: String,
    pub outputs: Vec<OutputPort>,
    pub inputs: Vec<InputPort>,
    #[serde(default)]
    pub config: JsonValue,
    #[serde(skip)]
    pub wasm: Option<WasmNode>,
    #[serde(default)]
    pub instruction_info: Option<InstructionInfo>,
}

impl NodeData {
    pub fn cmd_inputs(&self) -> Vec<CmdInputDescription> {
        self.inputs.iter().cloned().map(Into::into).collect()
    }
    pub fn cmd_outputs(&self) -> Vec<CmdOutputDescription> {
        self.outputs.iter().cloned().map(Into::into).collect()
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
        let config = JsonValue::Object(v2.config.into_iter().collect::<JsonMap<_, _>>());
        Self {
            r#type: infer_command_type(&v2.node_id),
            node_id: v2.node_id,
            outputs: v2.ports.outputs,
            inputs: v2.ports.inputs,
            config,
            wasm: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that serializing NodeData with OutputPort/InputPort (tooltip: None)
    /// does NOT emit "tooltip" in the JSON — preserving wire-format compatibility
    /// with Deno subprocesses and CapNP-RPC.
    #[test]
    fn node_data_serialization_omits_tooltip() {
        let nd = NodeData {
            r#type: CommandType::Native,
            node_id: "test".into(),
            outputs: vec![OutputPort {
                id: Uuid::nil(),
                name: "out".into(),
                r#type: ValueType::Free,
                optional: false,
                tooltip: None,
            }],
            inputs: vec![InputPort {
                id: Uuid::nil(),
                name: "in".into(),
                type_bounds: vec![ValueType::Free],
                required: true,
                passthrough: false,
                tooltip: None,
            }],
            config: JsonValue::Object(<_>::default()),
            wasm: None,
            instruction_info: None,
        };
        let json = serde_json::to_string(&nd).unwrap();
        assert!(
            !json.contains("tooltip"),
            "tooltip must not appear in wire format, got: {json}"
        );
    }

    /// Verify round-trip: serialize then deserialize produces identical NodeData.
    #[test]
    fn node_data_serde_round_trip() {
        let nd = NodeData {
            r#type: CommandType::Native,
            node_id: "round_trip".into(),
            outputs: vec![OutputPort {
                id: Uuid::nil(),
                name: "sig".into(),
                r#type: ValueType::Signature,
                optional: false,
                tooltip: None,
            }],
            inputs: vec![InputPort {
                id: Uuid::nil(),
                name: "fee_payer".into(),
                type_bounds: vec![ValueType::Keypair],
                required: true,
                passthrough: true,
                tooltip: None,
            }],
            config: JsonValue::Object(<_>::default()),
            wasm: None,
            instruction_info: None,
        };
        let json = serde_json::to_string(&nd).unwrap();
        let deserialized: NodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(nd, deserialized);
    }

    /// Verify V2 wire format: direct field names, no legacy nesting.
    #[test]
    fn wire_format_uses_v2_keys() {
        let nd = NodeData {
            r#type: CommandType::Native,
            node_id: "wire_test".into(),
            outputs: vec![OutputPort {
                id: Uuid::nil(),
                name: "out".into(),
                r#type: ValueType::Free,
                optional: false,
                tooltip: None,
            }],
            inputs: vec![InputPort {
                id: Uuid::nil(),
                name: "in".into(),
                type_bounds: vec![ValueType::Free],
                required: true,
                passthrough: false,
                tooltip: None,
            }],
            config: JsonValue::Object(<_>::default()),
            wasm: None,
            instruction_info: None,
        };
        let json = serde_json::to_string(&nd).unwrap();

        // V2 wire format uses direct field names
        assert!(json.contains("\"outputs\""), "missing 'outputs' key: {json}");
        assert!(json.contains("\"inputs\""), "missing 'inputs' key: {json}");
        assert!(json.contains("\"config\""), "missing 'config' key: {json}");

        // Legacy V1 keys must NOT appear
        assert!(
            !json.contains("\"sources\""),
            "'sources' must not be a JSON key: {json}"
        );
        assert!(
            !json.contains("\"targets\""),
            "'targets' must not be a JSON key: {json}"
        );
        assert!(
            !json.contains("\"form_data\""),
            "'form_data' must not be a JSON key: {json}"
        );
        assert!(
            !json.contains("\"targets_form\""),
            "'targets_form' must not be a JSON key: {json}"
        );

        // wasm fields must not appear (runtime-only, serde(skip))
        assert!(
            !json.contains("wasm"),
            "wasm must not appear: {json}"
        );
    }
}
