use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use strum::EnumIter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Native,
    Wasm,
    Deno,
    Mock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter)]
pub enum ValueType {
    #[serde(rename = "bool")]
    Bool,
    #[serde(rename = "u8")]
    U8,
    #[serde(rename = "u16")]
    U16,
    #[serde(rename = "u32")]
    U32,
    #[serde(rename = "u64")]
    U64,
    #[serde(rename = "u128")]
    U128,
    #[serde(rename = "i8")]
    I8,
    #[serde(rename = "i16")]
    I16,
    #[serde(rename = "i32")]
    I32,
    #[serde(rename = "i64")]
    I64,
    #[serde(rename = "i128")]
    I128,
    #[serde(rename = "f32")]
    F32,
    #[serde(rename = "f64")]
    F64,
    #[serde(rename = "decimal")]
    #[serde(alias = "number")]
    Decimal,
    #[serde(rename = "pubkey")]
    Pubkey,
    #[serde(rename = "keypair")]
    Keypair,
    #[serde(rename = "signature")]
    Signature,
    #[serde(rename = "address")]
    Address,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "bytes")]
    Bytes,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Map,
    #[serde(rename = "free")]
    Free,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputPort {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPort {
    pub name: String,
    #[serde(default = "default_free")]
    pub r#type: ValueType,
    #[serde(default)]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
}

const fn default_free() -> ValueType {
    ValueType::Free
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ports {
    #[serde(default)]
    pub inputs: Vec<InputPort>,
    #[serde(default)]
    pub outputs: Vec<OutputPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Permissions {
    #[serde(default)]
    pub user_tokens: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDefinition {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema_uri: Option<String>,
    pub version: String,
    pub name: String,
    pub r#type: NodeType,
    pub author_handle: String,
    pub source_code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub ports: Ports,
    #[serde(default = "default_empty_object")]
    pub config_schema: JsonValue,
    #[serde(default = "default_empty_object")]
    pub config: JsonValue,
    #[serde(default, skip_serializing_if = "permissions_is_default")]
    pub permissions: Permissions,
}

fn default_empty_object() -> JsonValue {
    JsonValue::Object(Default::default())
}

fn permissions_is_default(p: &Permissions) -> bool {
    !p.user_tokens
}

impl NodeDefinition {
    /// Returns the `@author/name` slug used as the runtime node identifier.
    pub fn slug(&self) -> String {
        format!("@{}/{}", self.author_handle, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn print_types() {
        let list = ValueType::iter().collect::<Vec<_>>();
        let s = serde_json::to_string_pretty(&list).unwrap();
        println!("{s}");
    }

    #[test]
    fn roundtrip_node_definition() {
        let json = r#"{
          "$schema": "https://schema.spaceoperator.com/node-v2.schema.json",
          "version": "2.0.0",
          "name": "const",
          "type": "native",
          "author_handle": "spo",
          "source_code": "crates/cmds-std/src/const_cmd.rs",
          "ports": {
            "inputs": [],
            "outputs": [
              { "name": "output", "type": "free", "optional": false }
            ]
          },
          "config_schema": {
            "type": "object",
            "properties": {
              "type": { "type": "string", "enum": ["JSON", "File"] },
              "static": { "type": "boolean" }
            },
            "required": ["type"],
            "additionalProperties": false
          },
          "config": {
            "type": { "S": "JSON" },
            "static": { "B": true }
          }
        }"#;

        let def: NodeDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.name, "const");
        assert_eq!(def.r#type, NodeType::Native);
        assert_eq!(def.author_handle, "spo");
        assert_eq!(def.slug(), "@spo/const");
        assert_eq!(def.ports.outputs.len(), 1);
        assert_eq!(def.ports.outputs[0].r#type, ValueType::Free);
        assert!(!def.ports.outputs[0].optional);
        assert!(def.ports.inputs.is_empty());

        // Re-serialize and re-parse
        let reserialized = serde_json::to_string_pretty(&def).unwrap();
        let def2: NodeDefinition = serde_json::from_str(&reserialized).unwrap();
        assert_eq!(def2.slug(), "@spo/const");
    }

    #[test]
    fn input_port_fields() {
        let json = r#"{
          "version": "2.0.0",
          "name": "transfer_sol",
          "type": "native",
          "author_handle": "spo",
          "source_code": "crates/cmds-solana/src/transfer_sol.rs",
          "ports": {
            "inputs": [
              { "name": "fee_payer", "type_bounds": ["keypair"], "required": true, "passthrough": true },
              { "name": "amount", "type_bounds": ["u64", "decimal"], "required": true },
              { "name": "memo", "type_bounds": ["string"] }
            ],
            "outputs": [
              { "name": "signature", "type": "signature" }
            ]
          },
          "config_schema": {},
          "config": {}
        }"#;

        let def: NodeDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.ports.inputs.len(), 3);

        assert_eq!(def.ports.inputs[0].name, "fee_payer");
        assert_eq!(def.ports.inputs[0].type_bounds, vec![ValueType::Keypair]);
        assert!(def.ports.inputs[0].required);
        assert!(def.ports.inputs[0].passthrough);

        assert_eq!(def.ports.inputs[1].name, "amount");
        assert_eq!(
            def.ports.inputs[1].type_bounds,
            vec![ValueType::U64, ValueType::Decimal]
        );
        assert!(def.ports.inputs[1].required);
        assert!(!def.ports.inputs[1].passthrough);

        assert_eq!(def.ports.inputs[2].name, "memo");
        assert!(!def.ports.inputs[2].required);

        assert_eq!(def.ports.outputs[0].r#type, ValueType::Signature);
    }

    #[test]
    fn number_alias_for_decimal() {
        let json = r#"{ "name": "x", "type_bounds": ["number"] }"#;
        let port: InputPort = serde_json::from_str(json).unwrap();
        assert_eq!(port.type_bounds, vec![ValueType::Decimal]);
    }

    #[test]
    fn permissions_parsed() {
        let json = r#"{
          "version": "2.0.0",
          "name": "perm_test",
          "type": "native",
          "author_handle": "spo",
          "source_code": "src/perm.rs",
          "permissions": { "user_tokens": true },
          "ports": { "inputs": [], "outputs": [] },
          "config_schema": {},
          "config": {}
        }"#;

        let def: NodeDefinition = serde_json::from_str(json).unwrap();
        assert!(def.permissions.user_tokens);
    }
}
