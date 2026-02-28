//! Get Block - Fetch complete block by slot
//!
//! Uses the Solana RPC `getBlock` method.

use crate::prelude::*;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;

pub const NAME: &str = "helius_get_block";
const DEFINITION: &str = flow_lib::node_definition!("helius/helius_get_block.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub url: String,
    pub slot: u64,
    #[serde(default = "default_encoding")]
    pub encoding: String,
    #[serde(default)]
    pub max_supported_version: Option<u8>,
}

fn default_encoding() -> String { "json".to_string() }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub block: JsonValue,
    pub block_height: Option<u64>,
    pub block_time: Option<i64>,
    pub transactions: Option<JsonValue>,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut config = json!({
        "encoding": input.encoding,
        "transactionDetails": "full",
        "rewards": false,
    });

    if let Some(version) = input.max_supported_version {
        config["maxSupportedTransactionVersion"] = json!(version);
    }

    let body = json!({
        "jsonrpc": "2.0",
        "method": "getBlock",
        "params": [input.slot, config],
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

    let block_height = result.get("blockHeight").and_then(|v| v.as_u64());
    let block_time = result.get("blockTime").and_then(|v| v.as_i64());
    let transactions = result.get("transactions").cloned();

    Ok(Output {
        block: result,
        block_height,
        block_time,
        transactions,
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
