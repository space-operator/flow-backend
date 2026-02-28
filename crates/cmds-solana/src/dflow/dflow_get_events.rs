//! DFlow Get Events - List prediction market events with filtering and pagination.
//!
//! DFlow Metadata API: GET /api/v1/events

use crate::prelude::*;
use serde_json::json;

pub const NAME: &str = "dflow_get_events";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_get_events.jsonc");

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
    pub series_tickers: Option<String>,
    pub status: Option<String>,
    pub sort: Option<String>,
    pub with_nested_markets: Option<bool>,
    pub is_initialized: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub events: JsonValue,
    pub cursor: Option<u64>,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://dev-prediction-markets-api.dflow.net/api/v1/events".to_string();

    let mut query: Vec<(&str, String)> = Vec::new();
    if let Some(ref val) = input.limit {
        query.push(("limit", val.to_string()));
    }
    if let Some(ref val) = input.cursor {
        query.push(("cursor", val.to_string()));
    }
    if let Some(ref val) = input.series_tickers {
        query.push(("series_tickers", val.to_string()));
    }
    if let Some(ref val) = input.status {
        query.push(("status", val.to_string()));
    }
    if let Some(ref val) = input.sort {
        query.push(("sort", val.to_string()));
    }
    if let Some(ref val) = input.with_nested_markets {
        query.push(("with_nested_markets", val.to_string()));
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

    let events = response.get("events").cloned().unwrap_or(json!(null));
    let cursor = response.get("cursor").and_then(|v| v.as_u64());

    Ok(Output { events, cursor })
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
        let json_str = include_str!("fixtures/events_list.json");
        let _parsed: crate::dflow::response_types::EventListResponse = serde_json::from_str(json_str)
            .expect("Failed to deserialize events_list.json");
    }

    #[tokio::test]
    #[ignore] // Hits live dev endpoint; run with: cargo test -- --ignored
    async fn test_run_get_events() {
        let api_key = std::env::var("DFLOW_API_KEY").unwrap_or_default();
        let input = Input {
            api_key,
            limit: Some(2),
            cursor: None,
            series_tickers: None,
            status: None,
            sort: None,
            with_nested_markets: None,
            is_initialized: None,
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
    }
}
