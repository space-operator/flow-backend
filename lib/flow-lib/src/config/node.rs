//! Node-definition transport parsing used by command builders.
//!
//! This module supports both:
//! - legacy command definitions (`data/sources/targets`)
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
        let sources = v2
            .ports
            .outputs
            .into_iter()
            .map(|o| Source {
                name: o.name,
                r#type: o.r#type,
                optional: o.optional,
            })
            .collect();
        let targets = v2
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
            sources,
            targets,
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
        assert_eq!(parsed.sources.len(), 1);
        assert!(parsed.targets.is_empty());
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
        assert_eq!(parsed.sources.len(), 1);
        assert!(parsed.targets.is_empty());
    }
}
