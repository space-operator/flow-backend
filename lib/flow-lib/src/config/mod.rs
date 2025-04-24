use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use std::{collections::HashMap, num::NonZeroU64, str::FromStr, sync::LazyLock, time::Duration};
use thiserror::Error as ThisError;
use uuid::Uuid;

use self::client::Network;

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
    #[serde(rename = "deno")]
    Deno,
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
    #[serde(default = "value::default::bool_false")]
    pub optional: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    pub environment: HashMap<String, String>,
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
                url: SolanaNet::Devnet.url(),
                cluster: SolanaNet::Devnet,
            },
            environment: <_>::default(),
            endpoints: <_>::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HttpClientConfig {
    pub timeout_in_secs: NonZeroU64,
    pub gzip: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SolanaClientConfig {
    pub url: String,
    pub cluster: SolanaNet,
}

impl SolanaClientConfig {
    pub fn build_client(&self) -> RpcClient {
        RpcClient::new_with_timeouts_and_commitment(
            self.url.clone(),
            Duration::from_secs(30),
            CommitmentConfig {
                commitment: CommitmentLevel::Finalized,
            },
            Duration::from_secs(180),
        )
    }
}

impl From<Network> for SolanaClientConfig {
    fn from(value: Network) -> Self {
        Self {
            url: value.url,
            cluster: value.cluster,
        }
    }
}

impl Default for SolanaClientConfig {
    fn default() -> Self {
        let cluster = SolanaNet::Devnet;
        Self {
            url: cluster.url().to_owned(),
            cluster,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    pub fn url(&self) -> String {
        match self {
            SolanaNet::Devnet => {
                static URL: LazyLock<String> = LazyLock::new(|| {
                    std::env::var("SOLANA_DEVNET_URL")
                        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_owned())
                });
                URL.clone()
            }
            SolanaNet::Testnet => {
                static URL: LazyLock<String> = LazyLock::new(|| {
                    std::env::var("SOLANA_TESTNET_URL")
                        .unwrap_or_else(|_| "https://api.testnet.solana.com".to_owned())
                });
                URL.clone()
            }
            SolanaNet::Mainnet => {
                static URL: LazyLock<String> = LazyLock::new(|| {
                    std::env::var("SOLANA_MAINNET_URL")
                        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_owned())
                });
                URL.clone()
            }
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SolanaNet::Devnet => "devnet",
            SolanaNet::Testnet => "testnet",
            SolanaNet::Mainnet => "mainnet-beta",
        }
    }

    pub fn from_url(url: &str) -> Result<Self, UnknownNetwork> {
        if url.contains("devnet") {
            Ok(SolanaNet::Devnet)
        } else if url.contains("testnet") {
            Ok(SolanaNet::Testnet)
        } else if url.contains("mainnet") {
            Ok(SolanaNet::Mainnet)
        } else {
            Err(UnknownNetwork(url.to_owned()))
        }
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
}
