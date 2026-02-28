//! Get Signatures For Address - Fetch transaction signatures
//!
//! Uses the Solana RPC `getSignaturesForAddress` method.

use crate::prelude::*;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;

pub const NAME: &str = "helius_get_signatures_for_address";
const DEFINITION: &str = flow_lib::node_definition!("helius/helius_get_signatures_for_address.jsonc");

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
    pub before: Option<String>,
    pub until: Option<String>,
}

fn default_limit() -> u32 { 1000 }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub signatures: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut config = json!({
        "limit": input.limit,
    });

    if let Some(before) = &input.before {
        config["before"] = json!(before);
    }

    if let Some(until) = &input.until {
        config["until"] = json!(until);
    }

    let body = json!({
        "jsonrpc": "2.0",
        "method": "getSignaturesForAddress",
        "params": [input.address.to_string(), config],
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
    let signatures = response.get("result").cloned().unwrap_or(json!([]));

    Ok(Output { signatures })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
