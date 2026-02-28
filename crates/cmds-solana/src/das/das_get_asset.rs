//! DAS Get Asset - Fetch a single digital asset by ID
//!
//! Uses the Metaplex DAS API `getAsset` method.

use crate::prelude::*;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;

pub const NAME: &str = "das_get_asset";
const DEFINITION: &str = flow_lib::node_definition!("das/das_get_asset.jsonc");

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
    pub asset: JsonValue,
    #[serde(default, with = "value::pubkey::opt")]
    pub owner: Option<Pubkey>,
    #[serde(default, with = "value::pubkey::opt")]
    pub collection: Option<Pubkey>,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let params = json!({ "id": input.asset_id.to_string() });
    let cache_key = super::cache::key("getAsset", &params);

    let result = if let Some(cached) = super::cache::get(&cache_key) {
        cached
    } else {
        let body = json!({
            "jsonrpc": "2.0",
            "method": "getAsset",
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
        let result = response
            .get("result")
            .cloned()
            .unwrap_or(JsonValue::Null);
        super::cache::set(cache_key, result.clone());
        result
    };

    // Parse owner from result.ownership.owner
    let owner = result
        .get("ownership")
        .and_then(|o| o.get("owner"))
        .and_then(|o| o.as_str())
        .and_then(|s| s.parse().ok());

    // Parse collection from result.grouping[].group_value where group_key == "collection"
    let collection = result
        .get("grouping")
        .and_then(|g| g.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|g| g.get("group_key").and_then(|k| k.as_str()) == Some("collection"))
                .and_then(|g| g.get("group_value"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
        });

    Ok(Output {
        asset: result,
        owner,
        collection,
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
