//! Jupiter Swap - Get unsigned swap transaction from quote
//!
//! Jupiter API: POST /swap/v1/swap

use crate::prelude::*;
use serde_json::json;

pub const NAME: &str = "jupiter_swap";
const DEFINITION: &str = flow_lib::node_definition!("jupiter/jupiter_swap.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    #[serde(rename = "userPublicKey")]
    pub user_public_key: String,
    #[serde(rename = "quoteResponse")]
    pub quote_response: JsonValue,
    #[serde(default, rename = "dynamicComputeUnitLimit")]
    pub dynamic_compute_unit_limit: Option<bool>,
    #[serde(default, rename = "dynamicSlippage")]
    pub dynamic_slippage: Option<bool>,
    #[serde(default, rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: Option<JsonValue>,
    #[serde(default, rename = "feeAccount")]
    pub fee_account: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://api.jup.ag/swap/v1/swap".to_string();

    let mut body = json!({
        "userPublicKey": input.user_public_key,
        "quoteResponse": input.quote_response,
    });
    if let Some(v) = input.dynamic_compute_unit_limit {
        body["dynamicComputeUnitLimit"] = json!(v);
    }
    if let Some(v) = input.dynamic_slippage {
        body["dynamicSlippage"] = json!(v);
    }
    if let Some(v) = input.prioritization_fee_lamports {
        body["prioritizationFeeLamports"] = v;
    }
    if let Some(v) = input.fee_account {
        body["feeAccount"] = json!(v);
    }

    let req = ctx
        .http()
        .post(&url)
        .header("x-api-key", &input.api_key)
        .header("Content-Type", "application/json")
        .json(&body);

    let resp = req.send().await?;

    if !resp.status().is_success() {
        return Err(CommandError::msg(format!(
            "Jupiter API error: {} {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }

    let response: JsonValue = resp.json().await?;

    Ok(Output { result: response })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "api_key" => "test-api-key",
            "userPublicKey" => "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
            "quoteResponse" => serde_json::json!({}),
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
