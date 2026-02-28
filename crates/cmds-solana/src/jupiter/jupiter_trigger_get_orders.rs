//! Jupiter Trigger Get Orders - Get trigger orders for account
//!
//! Jupiter API: GET /trigger/v1/getTriggerOrders

use crate::prelude::*;

pub const NAME: &str = "jupiter_trigger_get_orders";
const DEFINITION: &str = flow_lib::node_definition!("jupiter/jupiter_trigger_get_orders.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub user: String,
    #[serde(rename = "orderStatus")]
    pub order_status: String,
    #[serde(default)]
    pub page: Option<String>,
    #[serde(default, rename = "includeFailedTx")]
    pub include_failed_tx: Option<String>,
    #[serde(default, rename = "inputMint")]
    pub input_mint: Option<String>,
    #[serde(default, rename = "outputMint")]
    pub output_mint: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://api.jup.ag/trigger/v1/getTriggerOrders".to_string();

    let mut query: Vec<(&str, String)> = Vec::new();
    query.push(("user", input.user.clone()));
    query.push(("orderStatus", input.order_status.clone()));
    if let Some(ref v) = input.page {
        query.push(("page", v.clone()));
    }
    if let Some(ref v) = input.include_failed_tx {
        query.push(("includeFailedTx", v.clone()));
    }
    if let Some(ref v) = input.input_mint {
        query.push(("inputMint", v.clone()));
    }
    if let Some(ref v) = input.output_mint {
        query.push(("outputMint", v.clone()));
    }

    let req = ctx
        .http()
        .get(&url)
        .header("x-api-key", &input.api_key)
        .query(&query);

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
            "user" => "test-value",
            "orderStatus" => "test-value",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[tokio::test]
    #[ignore = "requires JUPITER_API_KEY"]
    async fn test_run_trigger_get_orders() {
        let api_key = match std::env::var("JUPITER_API_KEY") {
            Ok(k) => k,
            Err(_) => { eprintln!("JUPITER_API_KEY not set, skipping"); return; }
        };
        let input = Input {
            api_key,
            user: "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string(),
            order_status: "active".to_string(),
            page: None,
            include_failed_tx: None,
            input_mint: None,
            output_mint: None,
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
    }
}
