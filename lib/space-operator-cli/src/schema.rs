use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstructionInfo {
    pub before: Vec<String>,
    pub signature: String,
    pub after: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, strum::EnumString, strum::EnumIter, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ValueType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    #[strum(serialize = "number")]
    #[strum(serialize = "decimal")]
    Decimal,
    Pubkey,
    // Wormhole address
    Address,
    Keypair,
    Signature,
    String,
    Bytes,
    #[strum(serialize = "array")]
    #[strum(serialize = "list")]
    Array,
    #[strum(serialize = "object")]
    #[strum(serialize = "map")]
    Map,
    Json,
    Free,
}

// ID in node_definitions table (UUID)
pub type CommandId = uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Ports {
    #[serde(default)]
    pub inputs: Vec<Target>,
    #[serde(default)]
    pub outputs: Vec<Source>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CommandDefinition {
    pub r#type: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub ports: Ports,
    #[serde(default)]
    pub config: JsonValue,
    #[serde(default)]
    pub config_schema: JsonValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification: Option<JsonValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_version: Option<JsonValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal: Option<JsonValue>,
}

impl CommandDefinition {
    /// Returns the `@author/name` slug used as the runtime node identifier.
    pub fn slug(&self) -> String {
        match &self.author_handle {
            Some(author) => format!("@{}/{}", author, self.name),
            None => self.name.clone(),
        }
    }
}

fn default_version() -> String {
    "0.1".to_owned()
}

// --- Metadata types used when constructing config JSON ---

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RelatedTo {
    pub id: String,
    pub r#type: String,
    pub relationship: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Resources {
    pub source_code_url: String,
    pub documentation_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Usage {
    pub license: String,
    pub license_url: String,
    pub pricing: Pricing,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Pricing {
    pub currency: String,
    pub purchase_price: u64,
    pub price_per_run: u64,
    pub custom: Option<CustomPricing>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CustomPricing {
    pub unit: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Author {
    pub name: String,
    pub contact: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Design {
    pub width: u64,
    pub height: u64,
    pub icon_url: String,
    #[serde(rename = "backgroundColor")]
    pub background_color: String,
    #[serde(rename = "backgroundColorDark")]
    pub background_color_dark: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub name: String,
    pub r#type: String,
    #[serde(rename = "defaultValue", default)]
    pub default_value: JsonValue,
    #[serde(default)]
    pub tooltip: String,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub name: String,
    #[serde(default)]
    pub type_bounds: Vec<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(rename = "defaultValue", default)]
    pub default_value: JsonValue,
    #[serde(default)]
    pub tooltip: String,
    #[serde(default)]
    pub passthrough: bool,
}
