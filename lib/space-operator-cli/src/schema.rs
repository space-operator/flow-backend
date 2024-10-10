use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ID in database
pub type CommandId = i64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Data {
    pub node_definition_version: Option<String>,
    pub unique_id: Option<String>,
    pub node_id: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    pub tags: Option<Vec<String>>,
    pub related_to: Option<Vec<RelatedTo>>,
    pub resources: Option<Resources>,
    pub usage: Option<Usage>,
    pub authors: Option<Vec<Author>>,
    pub design: Option<Design>,
    pub options: Option<serde_json::Value>,
}

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
    #[serde(rename = "defaultValue")]
    pub default_value: JsonValue,
    pub tooltip: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub name: String,
    pub type_bounds: Vec<String>,
    pub required: bool,
    #[serde(rename = "defaultValue")]
    pub default_value: JsonValue,
    pub tooltip: String,
    pub passthrough: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CommandDefinition {
    pub r#type: String,
    pub data: Data,
    pub sources: Vec<Source>,
    pub targets: Vec<Target>,
    #[serde(rename = "targets_form.ui_schema")]
    pub ui_schema: JsonValue,
    #[serde(rename = "targets_form.json_schema")]
    pub json_schema: JsonValue,
}
