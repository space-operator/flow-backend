//! Get Transactions For Address - Helius enhanced transaction history
//!
//! Uses the Helius-specific `getTransactionsForAddress` method with
//! time-based filtering, bidirectional sorting, and full tx data.

use crate::prelude::*;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;

pub const NAME: &str = "helius_get_transactions_for_address";
const DEFINITION: &str = flow_lib::node_definition!("helius/helius_get_transactions_for_address.jsonc");

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
    pub address: Pubkey,
    #[serde(default = "default_limit")]
    pub limit: u32,
    pub before_signature: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    #[serde(default = "default_sort_order")]
    pub sort_order: String,
    #[serde(default = "default_include_token_accounts")]
    pub include_token_accounts: bool,
}

fn default_limit() -> u32 { 100 }
fn default_sort_order() -> String { "desc".to_string() }
fn default_include_token_accounts() -> bool { true }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub transactions: JsonValue,
    pub oldest_signature: Option<String>,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut params = json!({
        "address": input.address.to_string(),
        "limit": input.limit,
        "sortOrder": input.sort_order,
        "includeTokenAccounts": input.include_token_accounts,
    });

    if let Some(before) = &input.before_signature {
        params["beforeSignature"] = json!(before);
    }

    if let Some(start) = input.start_time {
        params["startTime"] = json!(start);
    }

    if let Some(end) = input.end_time {
        params["endTime"] = json!(end);
    }

    let body = json!({
        "jsonrpc": "2.0",
        "method": "getTransactionsForAddress",
        "params": params,
        "id": 1
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
            "RPC error: {} {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }

    let response: JsonValue = resp.json().await?;
    let result = response.get("result").cloned().unwrap_or(JsonValue::Null);

    // Extract transactions array
    let transactions = result.get("transactions").cloned()
        .or_else(|| if result.is_array() { Some(result.clone()) } else { None })
        .unwrap_or(json!([]));

    // Extract oldest signature for pagination
    let oldest_signature = transactions.as_array()
        .and_then(|arr| arr.last())
        .and_then(|tx| tx.get("signature"))
        .and_then(|s| s.as_str())
        .map(String::from);

    Ok(Output {
        transactions,
        oldest_signature,
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
