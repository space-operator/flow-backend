//! DFlow Create Swap - Create an imperative swap transaction from a quote response.
//!
//! DFlow Trading API: POST /swap
//!
//! **DEPRECATED**: Use `dflow_get_order` instead. The /quote and /swap endpoints
//! are replaced by the unified /order endpoint.

use crate::prelude::*;
use serde_json::json;

pub const NAME: &str = "dflow_create_swap";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_create_swap.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub quote_response: JsonValue,
    pub user_public_key: String,
    pub wrap_and_unwrap_sol: Option<bool>,
    pub fee_account: Option<String>,
    pub dynamic_compute_unit_limit: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub swap_transaction: String,
    pub last_valid_block_height: u64,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://quote-api.dflow.net/swap".to_string();

    let mut body = json!({
        "quote_response": input.quote_response,
        "user_public_key": input.user_public_key,
    });
    if let Some(val) = input.wrap_and_unwrap_sol {
        body["wrap_and_unwrap_sol"] = json!(val);
    }
    if let Some(val) = input.fee_account {
        body["fee_account"] = json!(val);
    }
    if let Some(val) = input.dynamic_compute_unit_limit {
        body["dynamic_compute_unit_limit"] = json!(val);
    }

    let resp = ctx
        .http()
        .post(&url)
        .header("x-api-key", &input.api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(CommandError::msg(format!(
            "DFlow API error: {} {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }

    let response: JsonValue = resp.json().await?;

    let swap_transaction = response.get("swap_transaction").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let last_valid_block_height = response.get("last_valid_block_height").and_then(|v| v.as_u64()).unwrap_or(0);

    Ok(Output { swap_transaction, last_valid_block_height })
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
            "quote_response" => serde_json::json!({}),
            "user_public_key" => "11111111111111111111111111111112",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
