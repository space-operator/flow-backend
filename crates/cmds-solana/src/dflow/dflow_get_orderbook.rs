//! DFlow Get Orderbook - Get orderbook depth for a prediction market by ticker.
//!
//! DFlow Metadata API: GET /api/v1/orderbook/{market_ticker}

use crate::prelude::*;

pub const NAME: &str = "dflow_get_orderbook";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_get_orderbook.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub market_ticker: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub orderbook: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = format!("https://dev-prediction-markets-api.dflow.net/api/v1/orderbook/{}", input.market_ticker);

    let query: Vec<(&str, String)> = Vec::new();


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

    let orderbook = response;

    Ok(Output { orderbook })
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
            "market_ticker" => "PRES-2024-KH",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_deserialize_response() {
        let json_str = include_str!("fixtures/orderbook.json");
        let _parsed: crate::dflow::response_types::Orderbook = serde_json::from_str(json_str)
            .expect("Failed to deserialize orderbook.json");
    }
}
