//! Get Inflation Reward - Fetch staking rewards for addresses
//!
//! Uses the Solana RPC `getInflationReward` method.

use crate::prelude::*;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;

pub const NAME: &str = "helius_get_inflation_reward";
const DEFINITION: &str = flow_lib::node_definition!("helius/helius_get_inflation_reward.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub url: String,
    pub addresses: JsonValue,
    pub epoch: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub rewards: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // Convert addresses to array of strings
    let addresses = match input.addresses {
        JsonValue::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<_>>(),
        JsonValue::String(s) => vec![s],
        _ => return Err(CommandError::msg("addresses must be an array of strings")),
    };

    let params = match input.epoch {
        Some(epoch) => json!([addresses, { "epoch": epoch }]),
        None => json!([addresses]),
    };

    let body = json!({
        "jsonrpc": "2.0",
        "method": "getInflationReward",
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
    let rewards = response.get("result").cloned().unwrap_or(json!([]));

    Ok(Output { rewards })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
