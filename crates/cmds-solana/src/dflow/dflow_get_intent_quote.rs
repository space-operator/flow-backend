//! DFlow Get Intent Quote - Get a declarative intent-based swap quote.
//!
//! DFlow Trading API: GET /intent

use crate::prelude::*;
use serde_json::json;

pub const NAME: &str = "dflow_get_intent_quote";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_get_intent_quote.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub input_mint: String,
    pub output_mint: String,
    pub amount: String,
    pub user_public_key: Option<String>,
    pub slippage_bps: Option<String>,
    pub platform_fee_bps: Option<u64>,
    pub fee_account: Option<String>,
    pub fee_budget: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub intent_quote: JsonValue,
    pub open_transaction: Option<String>,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://quote-api.dflow.net/intent".to_string();

    let mut query: Vec<(&str, String)> = Vec::new();
    query.push(("input_mint", input.input_mint.to_string()));
    query.push(("output_mint", input.output_mint.to_string()));
    query.push(("amount", input.amount.to_string()));
    if let Some(ref val) = input.user_public_key {
        query.push(("user_public_key", val.to_string()));
    }
    if let Some(ref val) = input.slippage_bps {
        query.push(("slippage_bps", val.to_string()));
    }
    if let Some(ref val) = input.platform_fee_bps {
        query.push(("platform_fee_bps", val.to_string()));
    }
    if let Some(ref val) = input.fee_account {
        query.push(("fee_account", val.to_string()));
    }
    if let Some(ref val) = input.fee_budget {
        query.push(("fee_budget", val.to_string()));
    }

    let resp = ctx
        .http()
        .get(&url)
        .header("x-api-key", &input.api_key)
        .query(&query)
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

    let intent_quote = response.get("intent_quote").cloned().unwrap_or(json!(null));
    let open_transaction = response.get("open_transaction").and_then(|v| v.as_str()).map(String::from);

    Ok(Output { intent_quote, open_transaction })
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
            "input_mint" => "So11111111111111111111111111111111111111112",
            "output_mint" => "So11111111111111111111111111111111111111112",
            "amount" => "1000000",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[tokio::test]
    #[ignore] // Hits live dev endpoint; run with: cargo test -- --ignored
    async fn test_run_get_intent_quote() {
        let api_key = match std::env::var("DFLOW_API_KEY") {
            Ok(k) => k,
            Err(_) => { eprintln!("DFLOW_API_KEY not set, skipping"); return; }
        };
        let input = Input {
            api_key,
            input_mint: "So11111111111111111111111111111111111111112".to_string(),
            output_mint: "So11111111111111111111111111111111111111112".to_string(),
            amount: "1000000".to_string(),
            user_public_key: None,
            slippage_bps: None,
            platform_fee_bps: None,
            fee_account: None,
            fee_budget: None,
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
    }
}
