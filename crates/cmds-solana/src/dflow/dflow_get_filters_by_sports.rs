//! DFlow Get Filters By Sports - Get filtering options organized by sports for prediction markets.
//!
//! DFlow Metadata API: GET /api/v1/filters_by_sports

use crate::prelude::*;
use serde_json::json;

pub const NAME: &str = "dflow_get_filters_by_sports";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_get_filters_by_sports.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub filters_by_sports: JsonValue,
    pub sport_ordering: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://dev-prediction-markets-api.dflow.net/api/v1/filters_by_sports".to_string();

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

    let filters_by_sports = response.get("filters_by_sports").cloned().unwrap_or(json!(null));
    let sport_ordering = response.get("sport_ordering").cloned().unwrap_or(json!(null));

    Ok(Output { filters_by_sports, sport_ordering })
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

    #[tokio::test]
    #[ignore] // Hits live dev endpoint; run with: cargo test -- --ignored
    async fn test_run_get_filters_by_sports() {
        let api_key = std::env::var("DFLOW_API_KEY").unwrap_or_default();
        let input = Input {
            api_key,
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
    }
}
