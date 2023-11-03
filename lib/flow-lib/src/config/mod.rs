use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::{collections::HashMap, num::NonZeroU64, str::FromStr};
use thiserror::Error as ThisError;
use uuid::Uuid;

pub mod client;
pub mod node;

/// Use to describe input types and output types of nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[serde(alias = "number")]
    #[serde(rename = "decimal")]
    Decimal,
    #[serde(rename = "pubkey")]
    Pubkey,
    // Wormhole address
    #[serde(rename = "address")]
    Address,
    #[serde(rename = "keypair")]
    Keypair,
    #[serde(rename = "signature")]
    Signature,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "bytes")]
    Bytes,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Map,
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "free")]
    Free,
    #[serde(other)]
    Other,
}

pub type FlowId = i32;
pub type NodeId = Uuid;
pub type FlowRunId = Uuid;

/// Command name and field name,
pub type Name = String;

/// Inputs and outputs of commands
pub type ValueSet = value::Map;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandType {
    #[serde(rename = "native")]
    Native,
    #[serde(rename = "mock")]
    Mock,
    #[serde(rename = "WASM")]
    Wasm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CmdInputDescription {
    pub name: Name,
    pub type_bounds: Vec<ValueType>,
    pub required: bool,
    pub passthrough: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CmdOutputDescription {
    pub name: Name,
    pub r#type: ValueType,
}

/// An input or output gate of a node
pub type Gate = (NodeId, Name);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowConfig {
    pub id: FlowId,
    pub ctx: ContextConfig,
    pub nodes: Vec<NodeConfig>,
    pub edges: Vec<(Gate, Gate)>,
    #[serde(default)]
    pub instructions_bundling: client::BundlingMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeConfig {
    pub id: NodeId,
    pub command_name: Name,
    pub form_data: JsonValue,
    pub client_node_data: client::NodeData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoints {
    pub flow_server: String,
    pub supabase: String,
    pub supabase_anon_key: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            flow_server: "http://localhost:8080".to_owned(),
            supabase: "http://localhost:8081".to_owned(),
            supabase_anon_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextConfig {
    pub http_client: HttpClientConfig,
    pub solana_client: SolanaClientConfig,
    pub environment: std::collections::HashMap<String, String>,
    pub endpoints: Endpoints,
}

impl Default for ContextConfig {
    fn default() -> Self {
        ContextConfig {
            http_client: HttpClientConfig {
                timeout_in_secs: NonZeroU64::new(100).unwrap(),
                gzip: true,
            },
            solana_client: SolanaClientConfig {
                url: "https://api.devnet.solana.com".to_owned(),
                cluster: SolanaNet::Devnet,
            },
            environment: <_>::default(),
            endpoints: <_>::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpClientConfig {
    pub timeout_in_secs: NonZeroU64,
    pub gzip: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolanaClientConfig {
    pub url: String,
    pub cluster: SolanaNet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolanaNet {
    #[serde(rename = "devnet")]
    Devnet,
    #[serde(rename = "testnet")]
    Testnet,
    #[serde(rename = "mainnet-beta")]
    Mainnet,
}

/// Unknown Sonana network.
#[derive(Debug, ThisError)]
#[error("unknown network: {0}")]
pub struct UnknownNetwork(pub String);

impl FromStr for SolanaNet {
    type Err = UnknownNetwork;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "devnet" => Ok(Self::Devnet),
            "testnet" => Ok(Self::Testnet),
            "mainnet-beta" => Ok(Self::Mainnet),
            s => Err(UnknownNetwork(s.to_owned())),
        }
    }
}

impl SolanaNet {
    pub fn url(&self) -> &'static str {
        match self {
            SolanaNet::Devnet => "https://api.devnet.solana.com",
            SolanaNet::Testnet => "https://api.testnet.solana.com",
            SolanaNet::Mainnet => "https://api.mainnet-beta.solana.com",
        }
    }

    pub fn from_url(url: &str) -> Result<Self, UnknownNetwork> {
        Ok(match url.strip_suffix('/') {
            Some("https://api.devnet.solana.com") => SolanaNet::Devnet,
            Some("https://api.testnet.solana.com") => SolanaNet::Testnet,
            Some("https://api.mainnet-beta.solana.com") => SolanaNet::Mainnet,
            _ => return Err(UnknownNetwork(url.to_owned())),
        })
    }
}

impl FlowConfig {
    pub fn new(config: client::ClientConfig) -> Self {
        fn get_name_from_id(names: &HashMap<Uuid, String>, id: &Uuid) -> Option<String> {
            match names.get(id) {
                Some(name) => Some(name.clone()),
                None => {
                    tracing::warn!("name not found for edge {}", id);
                    None
                }
            }
        }

        let source_names = config
            .nodes
            .iter()
            .flat_map(|n| n.data.sources.iter().map(|s| (s.id, s.name.clone())));
        let target_names = config
            .nodes
            .iter()
            .flat_map(|n| n.data.targets.iter().map(|s| (s.id, s.name.clone())));
        let names = source_names.chain(target_names).collect::<HashMap<_, _>>();

        let edges = config
            .edges
            .iter()
            .filter_map(|e| {
                let from: Gate = (e.source, get_name_from_id(&names, &e.source_handle.id)?);
                let to: Gate = (e.target, get_name_from_id(&names, &e.target_handle)?);
                Some((from, to))
            })
            .collect();

        let nodes = config
            .nodes
            .into_iter()
            .filter(|n| n.data.r#type != CommandType::Mock)
            .map(|n| NodeConfig {
                id: n.id,
                command_name: n.data.node_id.clone(),
                form_data: n.data.targets_form.form_data.clone(),
                client_node_data: n.data,
            })
            .collect();

        Self {
            id: config.id,
            ctx: ContextConfig {
                http_client: HttpClientConfig {
                    timeout_in_secs: NonZeroU64::new(100).unwrap(),
                    gzip: true,
                },
                solana_client: SolanaClientConfig {
                    url: config.sol_network.url,
                    cluster: config.sol_network.cluster,
                },
                environment: config.environment,
                endpoints: <_>::default(),
            },
            nodes,
            edges,
            instructions_bundling: config.instructions_bundling,
        }
    }

    pub fn parse_json(config: JsonValue) -> Result<Self, serde_json::Error> {
        let config: client::ClientConfig = serde_json::from_value(config)?;

        Ok(Self::new(config))
    }
}
