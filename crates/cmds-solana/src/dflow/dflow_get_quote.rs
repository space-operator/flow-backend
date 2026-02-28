//! DFlow Get Quote - Get an imperative swap quote with explicit route plan.
//!
//! DFlow Trading API: GET /quote
//!
//! **DEPRECATED**: Use `dflow_get_order` instead. The /quote and /swap endpoints
//! are replaced by the unified /order endpoint.

use crate::prelude::*;

pub const NAME: &str = "dflow_get_quote";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_get_quote.jsonc");

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
    pub slippage_bps: Option<String>,
    pub dexes: Option<String>,
    pub exclude_dexes: Option<String>,
    pub only_direct_routes: Option<bool>,
    pub platform_fee_bps: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub quote: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://quote-api.dflow.net/quote".to_string();

    let mut query: Vec<(&str, String)> = Vec::new();
    query.push(("input_mint", input.input_mint.to_string()));
    query.push(("output_mint", input.output_mint.to_string()));
    query.push(("amount", input.amount.to_string()));
    if let Some(ref val) = input.slippage_bps {
        query.push(("slippage_bps", val.to_string()));
    }
    if let Some(ref val) = input.dexes {
        query.push(("dexes", val.to_string()));
    }
    if let Some(ref val) = input.exclude_dexes {
        query.push(("exclude_dexes", val.to_string()));
    }
    if let Some(ref val) = input.only_direct_routes {
        query.push(("only_direct_routes", val.to_string()));
    }
    if let Some(ref val) = input.platform_fee_bps {
        query.push(("platform_fee_bps", val.to_string()));
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

    let quote = response;

    Ok(Output { quote })
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
}
