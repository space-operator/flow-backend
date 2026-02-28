//! DFlow Get Event - Get a single prediction market event with nested markets.
//!
//! DFlow Metadata API: GET /api/v1/event/{event_id}

use crate::prelude::*;

pub const NAME: &str = "dflow_get_event";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_get_event.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub event_id: String,
    pub with_nested_markets: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub event: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = format!("https://dev-prediction-markets-api.dflow.net/api/v1/event/{}", input.event_id);

    let mut query: Vec<(&str, String)> = Vec::new();
    if let Some(ref val) = input.with_nested_markets {
        query.push(("with_nested_markets", val.to_string()));
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

    let event = response;

    Ok(Output { event })
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
            "event_id" => "KXSB-26",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_deserialize_response() {
        let json_str = include_str!("fixtures/event.json");
        let _parsed: crate::dflow::response_types::Event = serde_json::from_str(json_str)
            .expect("Failed to deserialize event.json");
        assert!(!_parsed.ticker.is_empty(), "ticker should not be empty");
    }

    #[tokio::test]
    #[ignore] // Hits live dev endpoint; run with: cargo test -- --ignored
    async fn test_run_get_event() {
        let api_key = std::env::var("DFLOW_API_KEY").unwrap_or_default();
        let input = Input {
            api_key,
            event_id: "KXSB-26".to_string(),
            with_nested_markets: None,
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
    }
}
