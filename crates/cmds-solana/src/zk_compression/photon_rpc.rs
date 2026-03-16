//! Helper for calling Light Protocol's Photon JSON-RPC methods.
//!
//! These methods are supported by Helius RPC endpoints and other ZK Compression
//! compatible RPCs. We use `ctx.http()` (reqwest) and `ctx.solana_config().url`.

use serde::{Deserialize, Serialize};
use solana_pubkey::Pubkey;

// =============================================================================
// JSON-RPC Generic Types
// =============================================================================

#[derive(Serialize)]
struct JsonRpcRequest<P: Serialize> {
    jsonrpc: &'static str,
    id: &'static str,
    method: &'static str,
    params: P,
}

#[derive(Deserialize, Debug)]
struct JsonRpcResponse<T> {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    #[allow(dead_code)]
    id: Option<serde_json::Value>,
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize, Debug)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RPC error {}: {}", self.code, self.message)
    }
}

// =============================================================================
// getCompressedTokenAccountsByOwner
// =============================================================================

#[derive(Serialize)]
struct GetCompressedTokenAccountsParams {
    owner: String,
    mint: String,
}

#[derive(Deserialize, Debug)]
pub struct CompressedTokenAccountList {
    pub items: Vec<CompressedTokenAccountItem>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompressedTokenAccountItem {
    pub hash: String,
    pub leaf_index: u32,
    pub owner: String,
    #[serde(default)]
    pub lamports: u64,
    pub tree: String,
    pub seq: u64,
    pub token_data: CompressedTokenData,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompressedTokenData {
    pub mint: String,
    pub owner: String,
    pub amount: u64,
    pub delegate: Option<String>,
    pub state: String,
}

/// Fetch compressed token accounts for an owner, filtered by mint.
pub async fn get_compressed_token_accounts_by_owner(
    http: &reqwest::Client,
    rpc_url: &str,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<Vec<CompressedTokenAccountItem>, String> {
    let req = JsonRpcRequest {
        jsonrpc: "2.0",
        id: "zk-1",
        method: "getCompressedTokenAccountsByOwner",
        params: GetCompressedTokenAccountsParams {
            owner: owner.to_string(),
            mint: mint.to_string(),
        },
    };

    let resp = http
        .post(rpc_url)
        .json(&req)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {e}"))?;

    let body: JsonRpcResponse<CompressedTokenAccountList> = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {e}"))?;

    if let Some(err) = body.error {
        return Err(err.to_string());
    }

    Ok(body.result.map(|r| r.items).unwrap_or_default())
}

// =============================================================================
// getValidityProof
// =============================================================================

#[derive(Serialize)]
struct GetValidityProofParams {
    hashes: Vec<String>,
    #[serde(rename = "newAddressesWithTrees")]
    new_addresses_with_trees: Vec<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ValidityProofResponse {
    pub compressed_proof: ProofComponents,
    pub root_indices: Vec<u32>,
    pub leaf_indices: Vec<u32>,
    pub leaves: Vec<String>,
    pub merkle_trees: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct ProofComponents {
    pub a: String,
    pub b: String,
    pub c: String,
}

/// Fetch a validity proof for the given compressed account hashes.
pub async fn get_validity_proof(
    http: &reqwest::Client,
    rpc_url: &str,
    hashes: Vec<String>,
) -> Result<ValidityProofResponse, String> {
    let req = JsonRpcRequest {
        jsonrpc: "2.0",
        id: "zk-2",
        method: "getValidityProof",
        params: GetValidityProofParams {
            hashes,
            new_addresses_with_trees: vec![],
        },
    };

    let resp = http
        .post(rpc_url)
        .json(&req)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {e}"))?;

    let body: JsonRpcResponse<ValidityProofResponse> = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {e}"))?;

    if let Some(err) = body.error {
        return Err(err.to_string());
    }

    body.result.ok_or_else(|| "No result in response".to_string())
}

/// Parse a base58-encoded string to a v2 Pubkey.
pub fn parse_pubkey_v2(s: &str) -> Result<solana_program_v2::pubkey::Pubkey, String> {
    let pk: Pubkey = s.parse().map_err(|e| format!("Invalid pubkey '{s}': {e}"))?;
    Ok(solana_program_v2::pubkey::Pubkey::new_from_array(pk.to_bytes()))
}

/// Decode a base58-encoded proof component into a fixed-size byte array.
pub fn decode_proof_component<const N: usize>(s: &str) -> Result<[u8; N], String> {
    let bytes = bs58::decode(s)
        .into_vec()
        .map_err(|e| format!("Base58 decode error: {e}"))?;
    bytes
        .try_into()
        .map_err(|_| format!("Proof component wrong length, expected {N}"))
}
