//! Get Blocks - Fetch list of confirmed blocks in slot range
//!
//! Uses the Solana RPC `getBlocks` method.

use crate::prelude::*;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;

pub const NAME: &str = "helius_get_blocks";
const DEFINITION: &str = flow_lib::node_definition!("helius/helius_get_blocks.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub url: String,
    pub start_slot: u64,
    pub end_slot: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub blocks: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let params = match input.end_slot {
        Some(end) => json!([input.start_slot, end]),
        None => json!([input.start_slot]),
    };

    let body = json!({
        "jsonrpc": "2.0",
        "method": "getBlocks",
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
    let blocks = response.get("result").cloned().unwrap_or(json!([]));

    Ok(Output { blocks })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
