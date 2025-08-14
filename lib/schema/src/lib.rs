use serde::{Deserialize, Serialize};
use value::Value;
use strum::EnumIter;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum NodeType {
    Native,
    Wasm,
    Deno,
    Rhai,
    Interflow,
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumIter)]
pub enum TypeBound {
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
    Decimal,
    #[serde(rename = "pubkey")]
    Pubkey,
    #[serde(rename = "keypair")]
    Keypair,
    #[serde(rename = "signature")]
    Signature,
    #[serde(rename = "address")]
    WormholeAddress,
    #[serde(rename = "string")]
    String,
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
pub struct Input {
    pub name: String,
    pub type_bounds: Vec<TypeBound>,
    pub required: Option<bool>,
    pub passthrough: Option<bool>,
    pub value: Option<Value>,
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub name: String,
    pub r#type: TypeBound,
    pub required: Option<bool>,
    pub value: Option<Value>,
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionInfo {
    signature: Option<String>,
    after: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDefinition {
    pub r#type: NodeType,
    pub name: String,
    pub version: semver::Version,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub instruction_info: Option<InstructionInfo>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]    
    fn print_types() {
        let list = TypeBound::iter().collect::<Vec<_>>();
        let s = serde_json::to_string_pretty(&list).unwrap();
        println!("{s}");
    }
}
