//! Node-definition transport parsing used by command builders.
//!
//! This module supports both:
//! - legacy command definitions (`data/sources/targets` aliased to `outputs/inputs`)
//! - V2 node definitions (`name/ports/config_schema/config/...`)
//!
//! Note: only add fields that are needed in backend execution.
use crate::command::InstructionInfo;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::io;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Definition {
    pub r#type: super::CommandType,
    pub data: Data,
    #[serde(alias = "sources")]
    pub outputs: Vec<Source>,
    #[serde(alias = "targets")]
    pub inputs: Vec<Target>,
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

#[derive(Deserialize, Debug, Clone)]
struct DefinitionV2 {
    pub r#type: super::CommandType,
    pub name: String,
    pub author_handle: String,
    pub ports: PortsV2,
    #[serde(default)]
    pub permissions: Permissions,
}

#[derive(Deserialize, Debug, Clone)]
struct PortsV2 {
    pub inputs: Vec<InputPortV2>,
    pub outputs: Vec<OutputPortV2>,
}

#[derive(Deserialize, Debug, Clone)]
struct InputPortV2 {
    pub name: String,
    #[serde(default)]
    pub type_bounds: Vec<super::ValueType>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub passthrough: bool,
}

#[derive(Deserialize, Debug, Clone)]
struct OutputPortV2 {
    pub name: String,
    #[serde(default = "default_free_type")]
    pub r#type: super::ValueType,
    #[serde(default)]
    pub optional: bool,
}

const fn default_free_type() -> super::ValueType {
    super::ValueType::Free
}

impl From<DefinitionV2> for Definition {
    fn from(v2: DefinitionV2) -> Self {
        let outputs = v2
            .ports
            .outputs
            .into_iter()
            .map(|o| Source {
                name: o.name,
                r#type: o.r#type,
                optional: o.optional,
            })
            .collect();
        let inputs = v2
            .ports
            .inputs
            .into_iter()
            .map(|i| Target {
                name: i.name,
                type_bounds: i.type_bounds,
                required: i.required,
                passthrough: i.passthrough,
            })
            .collect();

        Self {
            r#type: v2.r#type,
            data: Data {
                node_id: format!("@{}/{}", v2.author_handle, v2.name),
                instruction_info: None,
            },
            outputs,
            inputs,
            permissions: v2.permissions,
        }
    }
}

fn parse_jsonc_value(def: &str) -> Result<JsonValue, serde_json::Error> {
    jsonc_parser::parse_to_serde_value(def, &Default::default())
        .map_err(|error| {
            serde_json::Error::io(io::Error::new(
                io::ErrorKind::InvalidData,
                error.to_string(),
            ))
        })?
        .ok_or_else(|| {
            serde_json::Error::io(io::Error::new(
                io::ErrorKind::InvalidData,
                "empty node definition",
            ))
        })
}

/// Parse a node definition that may be either legacy format or V2 format.
///
/// Input may be JSON or JSONC.
pub fn parse_definition(def: &str) -> Result<Definition, serde_json::Error> {
    let value = parse_jsonc_value(def)?;

    if let Ok(legacy) = serde_json::from_value::<Definition>(value.clone()) {
        return Ok(legacy);
    }

    serde_json::from_value::<DefinitionV2>(value).map(Into::into)
}

#[cfg(test)]
mod tests {
    use super::parse_definition;
    use crate::config::ValueType;

    #[test]
    fn parses_legacy_definition() {
        let input = r#"{
          "type": "native",
          "data": { "node_id": "const", "instruction_info": null },
          "sources": [{ "name": "output", "type": "free", "optional": false }],
          "targets": []
        }"#;

        let parsed = parse_definition(input).expect("legacy definition should parse");
        assert_eq!(parsed.data.node_id, "const");
        assert_eq!(parsed.outputs.len(), 1);
        assert!(parsed.inputs.is_empty());
    }

    #[test]
    fn parses_v2_jsonc_definition() {
        let input = r#"{
          // V2 node definition
          "version": "2.0.0",
          "name": "const",
          "author_handle": "spo",
          "type": "native",
          "ports": {
            "inputs": [],
            "outputs": [
              { "name": "output", "type": "free" }
            ]
          },
          "config_schema": {},
          "config": {}
        }"#;

        let parsed = parse_definition(input).expect("v2 jsonc definition should parse");
        assert_eq!(parsed.data.node_id, "@spo/const");
        assert_eq!(parsed.outputs.len(), 1);
        assert!(parsed.inputs.is_empty());
    }

    // ── Test 10: All input fields survive V2 conversion ────────────────

    #[test]
    fn v2_all_input_fields_survive_conversion() {
        let input = r#"{
          "version": "0.1",
          "name": "rich",
          "type": "native",
          "author_handle": "spo",
          "ports": {
            "inputs": [
              { "name": "fee_payer", "type_bounds": ["keypair"], "required": true, "passthrough": true },
              { "name": "amount", "type_bounds": ["u64", "decimal"], "required": true, "passthrough": false },
              { "name": "note", "type_bounds": ["string"], "required": false, "passthrough": false }
            ],
            "outputs": []
          }
        }"#;

        let parsed = parse_definition(input).unwrap();
        assert_eq!(parsed.inputs.len(), 3);

        assert_eq!(parsed.inputs[0].name, "fee_payer");
        assert_eq!(parsed.inputs[0].type_bounds, vec![ValueType::Keypair]);
        assert!(parsed.inputs[0].required);
        assert!(parsed.inputs[0].passthrough);

        assert_eq!(parsed.inputs[1].name, "amount");
        assert_eq!(parsed.inputs[1].type_bounds, vec![ValueType::U64, ValueType::Decimal]);
        assert!(parsed.inputs[1].required);
        assert!(!parsed.inputs[1].passthrough);

        assert_eq!(parsed.inputs[2].name, "note");
        assert_eq!(parsed.inputs[2].type_bounds, vec![ValueType::String]);
        assert!(!parsed.inputs[2].required);
        assert!(!parsed.inputs[2].passthrough);
    }

    // ── Test 11: Optional output survives conversion ───────────────────

    #[test]
    fn v2_optional_output_survives_conversion() {
        let input = r#"{
          "version": "0.1",
          "name": "opt_test",
          "type": "native",
          "author_handle": "spo",
          "ports": {
            "inputs": [],
            "outputs": [
              { "name": "required_out", "type": "signature" },
              { "name": "optional_out", "type": "free", "optional": true }
            ]
          }
        }"#;

        let parsed = parse_definition(input).unwrap();
        assert_eq!(parsed.outputs.len(), 2);

        assert_eq!(parsed.outputs[0].name, "required_out");
        assert_eq!(parsed.outputs[0].r#type, ValueType::Signature);
        assert!(!parsed.outputs[0].optional);

        assert_eq!(parsed.outputs[1].name, "optional_out");
        assert_eq!(parsed.outputs[1].r#type, ValueType::Free);
        assert!(parsed.outputs[1].optional);
    }

    // ── Test 12: node_id = @{author}/{name} ────────────────────────────

    #[test]
    fn v2_node_id_format_with_custom_author() {
        let input = r#"{
          "version": "0.1",
          "name": "my_tool",
          "type": "native",
          "author_handle": "myorg",
          "ports": { "inputs": [], "outputs": [] }
        }"#;

        let parsed = parse_definition(input).unwrap();
        assert_eq!(parsed.data.node_id, "@myorg/my_tool");
    }

    // ── Test 13: V2 conversion always sets instruction_info to None ────

    #[test]
    fn v2_instruction_info_always_none() {
        let input = r#"{
          "version": "0.1",
          "name": "no_instr",
          "type": "native",
          "author_handle": "spo",
          "ports": {
            "inputs": [],
            "outputs": [{ "name": "sig", "type": "signature" }]
          }
        }"#;

        let parsed = parse_definition(input).unwrap();
        assert!(parsed.data.instruction_info.is_none());
    }

    // ── Test 14: Permissions from JSONC ─────────────────────────────────

    #[test]
    fn v2_permissions_from_jsonc() {
        let input = r#"{
          "version": "0.1",
          "name": "perm_test",
          "type": "native",
          "author_handle": "spo",
          "permissions": { "user_tokens": true },
          "ports": { "inputs": [], "outputs": [] }
        }"#;

        let parsed = parse_definition(input).unwrap();
        assert!(parsed.permissions.user_tokens);
    }
}
