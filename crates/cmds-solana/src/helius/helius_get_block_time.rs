//! Get Block Time - Fetch estimated production time of a block
//!
//! Uses the Solana RPC `getBlockTime` method.

use crate::prelude::*;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;

pub const NAME: &str = "helius_get_block_time";
const DEFINITION: &str = flow_lib::node_definition!("helius/helius_get_block_time.jsonc");

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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub block_time: Option<i64>,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": "getBlockTime",
        "params": [input.slot],
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
    let block_time = response.get("result").and_then(|v| v.as_i64());

    Ok(Output { block_time })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
