//! DAS Get Asset Proof - Fetch Merkle proof for compressed NFT
//!
//! Uses the Metaplex DAS API `getAssetProof` method.

use crate::prelude::*;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;

pub const NAME: &str = "das_get_asset_proof";
const DEFINITION: &str = flow_lib::node_definition!("das/das_get_asset_proof.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub url: String,
    #[serde_as(as = "AsPubkey")]
    pub asset_id: Pubkey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub proof: JsonValue,
    pub root: Option<String>,
    pub leaf: Option<String>,
    #[serde(default, with = "value::pubkey::opt")]
    pub tree_id: Option<Pubkey>,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let params = json!({ "id": input.asset_id.to_string() });
    let cache_key = super::cache::key("getAssetProof", &params);

    let result = if let Some(cached) = super::cache::get(&cache_key) {
        cached
    } else {
        let body = json!({
            "jsonrpc": "2.0",
            "method": "getAssetProof",
            "params": params,
            "id": "space-operator"
        });

        let resp = ctx
            .http()
            .post(&input.url)
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(CommandError::msg(format!(
                "DAS API error: {} {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            )));
        }

        let response: JsonValue = resp.json().await?;
        let result = response.get("result").cloned().unwrap_or(JsonValue::Null);
        super::cache::set(cache_key, result.clone());
        result
    };

    let proof = result.get("proof").cloned().unwrap_or(json!([]));
    let root = result
        .get("root")
        .and_then(|r| r.as_str())
        .map(String::from);
    let leaf = result
        .get("leaf")
        .and_then(|l| l.as_str())
        .map(String::from);
    let tree_id = result
        .get("tree_id")
        .and_then(|t| t.as_str())
        .and_then(|s| s.parse().ok());

    Ok(Output {
        proof,
        root,
        leaf,
        tree_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
