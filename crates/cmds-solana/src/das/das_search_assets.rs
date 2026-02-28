//! DAS Search Assets - Search with multiple filter criteria
//!
//! Uses the Metaplex DAS API `searchAssets` method.

use crate::prelude::*;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;

pub const NAME: &str = "das_search_assets";
const DEFINITION: &str = flow_lib::node_definition!("das/das_search_assets.jsonc");

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
    #[serde(default)]
    #[serde_as(as = "Option<AsPubkey>")]
    pub owner: Option<Pubkey>,
    #[serde(default)]
    #[serde_as(as = "Option<AsPubkey>")]
    pub creator: Option<Pubkey>,
    #[serde(default)]
    #[serde_as(as = "Option<AsPubkey>")]
    pub collection: Option<Pubkey>,
    #[serde(default)]
    pub burnt: bool,
    pub frozen: Option<bool>,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_page() -> u32 { 1 }
fn default_limit() -> u32 { 100 }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub items: JsonValue,
    pub total: Option<u32>,
    pub page: Option<u32>,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut params = json!({
        "page": input.page,
        "limit": input.limit,
        "burnt": input.burnt,
    });

    if let Some(owner) = input.owner {
        params["ownerAddress"] = json!(owner.to_string());
    }

    if let Some(creator) = input.creator {
        params["creatorAddress"] = json!(creator.to_string());
    }

    if let Some(collection) = input.collection {
        params["grouping"] = json!([["collection", collection.to_string()]]);
    }

    if let Some(frozen) = input.frozen {
        params["frozen"] = json!(frozen);
    }

    let cache_key = super::cache::key("searchAssets", &params);

    let result = if let Some(cached) = super::cache::get(&cache_key) {
        cached
    } else {
        let body = json!({
            "jsonrpc": "2.0",
            "method": "searchAssets",
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

    let items = result.get("items").cloned().unwrap_or(json!([]));
    let total = result
        .get("total")
        .and_then(|t| t.as_u64())
        .map(|t| t as u32);
    let page = result
        .get("page")
        .and_then(|p| p.as_u64())
        .map(|p| p as u32);

    Ok(Output { items, total, page })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
