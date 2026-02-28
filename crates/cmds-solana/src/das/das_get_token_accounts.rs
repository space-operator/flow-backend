//! DAS Get Token Accounts - Fetch token accounts by owner or mint
//!
//! Uses the Metaplex DAS API `getTokenAccounts` method.

use crate::prelude::*;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;

pub const NAME: &str = "das_get_token_accounts";
const DEFINITION: &str = flow_lib::node_definition!("das/das_get_token_accounts.jsonc");

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
    pub mint: Option<Pubkey>,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_page() -> u32 { 1 }
fn default_limit() -> u32 { 100 }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub token_accounts: JsonValue,
    pub total: Option<u32>,
    pub page: Option<u32>,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Require at least one of owner or mint
    if input.owner.is_none() && input.mint.is_none() {
        return Err(CommandError::msg("Either owner or mint must be provided"));
    }

    let mut params = json!({
        "page": input.page,
        "limit": input.limit
    });

    if let Some(owner) = input.owner {
        params["owner"] = json!(owner.to_string());
    }

    if let Some(mint) = input.mint {
        params["mint"] = json!(mint.to_string());
    }

    let cache_key = super::cache::key("getTokenAccounts", &params);

    let result = if let Some(cached) = super::cache::get(&cache_key) {
        cached
    } else {
        let body = json!({
            "jsonrpc": "2.0",
            "method": "getTokenAccounts",
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

    let token_accounts = result.get("token_accounts").cloned().unwrap_or(json!([]));
    let total = result.get("total").and_then(|t| t.as_u64()).map(|t| t as u32);
    let page = result.get("page").and_then(|p| p.as_u64()).map(|p| p as u32);

    Ok(Output { token_accounts, total, page })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
