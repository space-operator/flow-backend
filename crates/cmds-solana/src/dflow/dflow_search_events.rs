//! DFlow Search Events - Search prediction market events by title or ticker.
//!
//! DFlow Metadata API: GET /api/v1/search

use crate::prelude::*;
use serde_json::json;

pub const NAME: &str = "dflow_search_events";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_search_events.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub q: String,
    pub limit: Option<u64>,
    pub cursor: Option<u64>,
    pub sort: Option<String>,
    pub order: Option<String>,
    pub with_nested_markets: Option<bool>,
    pub with_market_accounts: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub events: JsonValue,
    pub cursor: Option<u64>,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://dev-prediction-markets-api.dflow.net/api/v1/search".to_string();

    let mut query: Vec<(&str, String)> = Vec::new();
    query.push(("q", input.q.to_string()));
    if let Some(ref val) = input.limit {
        query.push(("limit", val.to_string()));
    }
    if let Some(ref val) = input.cursor {
        query.push(("cursor", val.to_string()));
    }
    if let Some(ref val) = input.sort {
        query.push(("sort", val.to_string()));
    }
    if let Some(ref val) = input.order {
        query.push(("order", val.to_string()));
    }
    if let Some(ref val) = input.with_nested_markets {
        query.push(("with_nested_markets", val.to_string()));
    }
    if let Some(ref val) = input.with_market_accounts {
        query.push(("with_market_accounts", val.to_string()));
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
            "q" => "test-value",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_deserialize_response() {
        let json_str = include_str!("fixtures/search.json");
        let _parsed: crate::dflow::response_types::EventListResponse = serde_json::from_str(json_str)
            .expect("Failed to deserialize search.json");
    }

    #[tokio::test]
    #[ignore] // Hits live dev endpoint; run with: cargo test -- --ignored
    async fn test_run_search_events() {
        let api_key = std::env::var("DFLOW_API_KEY").unwrap_or_default();
        let input = Input {
            api_key,
            q: "bitcoin".to_string(),
            limit: Some(2),
            cursor: None,
            sort: None,
            order: None,
            with_nested_markets: None,
            with_market_accounts: None,
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
    }
}
