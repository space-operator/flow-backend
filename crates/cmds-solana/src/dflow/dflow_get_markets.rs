//! DFlow Get Markets - List prediction markets with filtering, sorting, and pagination.
//!
//! DFlow Metadata API: GET /api/v1/markets

use crate::prelude::*;
use serde_json::json;

pub const NAME: &str = "dflow_get_markets";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_get_markets.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub limit: Option<u64>,
    pub cursor: Option<u64>,
    pub status: Option<String>,
    pub sort: Option<String>,
    pub is_initialized: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub markets: JsonValue,
    pub cursor: Option<u64>,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://dev-prediction-markets-api.dflow.net/api/v1/markets".to_string();

    let mut query: Vec<(&str, String)> = Vec::new();
    if let Some(ref val) = input.limit {
        query.push(("limit", val.to_string()));
    }
    if let Some(ref val) = input.cursor {
        query.push(("cursor", val.to_string()));
    }
    if let Some(ref val) = input.status {
        query.push(("status", val.to_string()));
    }
    if let Some(ref val) = input.sort {
        query.push(("sort", val.to_string()));
    }
    if let Some(ref val) = input.is_initialized {
        query.push(("is_initialized", val.to_string()));
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

    let markets = response.get("markets").cloned().unwrap_or(json!(null));
    let cursor = response.get("cursor").and_then(|v| v.as_u64());

    Ok(Output { markets, cursor })
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
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_deserialize_response() {
        let json_str = include_str!("fixtures/markets_list.json");
        let _parsed: crate::dflow::response_types::MarketListResponse = serde_json::from_str(json_str)
            .expect("Failed to deserialize markets_list.json");
    }

    #[tokio::test]
    #[ignore] // Hits live dev endpoint; run with: cargo test -- --ignored
    async fn test_run_get_markets() {
        let api_key = std::env::var("DFLOW_API_KEY").unwrap_or_default();
        let input = Input {
            api_key,
            limit: Some(2),
            cursor: None,
            status: None,
            sort: None,
            is_initialized: None,
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
    }
}
