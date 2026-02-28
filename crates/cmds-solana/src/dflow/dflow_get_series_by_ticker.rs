//! DFlow Get Series By Ticker - Get a single prediction market series by ticker.
//!
//! DFlow Metadata API: GET /api/v1/series/{series_ticker}

use crate::prelude::*;

pub const NAME: &str = "dflow_get_series_by_ticker";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_get_series_by_ticker.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub series_ticker: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub series: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = format!("https://dev-prediction-markets-api.dflow.net/api/v1/series/{}", input.series_ticker);

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

    let series = response;

    Ok(Output { series })
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
            "series_ticker" => "PRES-2024-KH",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_deserialize_response() {
        let json_str = include_str!("fixtures/series.json");
        let _parsed: crate::dflow::response_types::Series = serde_json::from_str(json_str)
            .expect("Failed to deserialize series.json");
        assert!(!_parsed.ticker.is_empty(), "ticker should not be empty");
    }

    #[tokio::test]
    #[ignore] // Hits live dev endpoint; run with: cargo test -- --ignored
    async fn test_run_get_series_by_ticker() {
        let api_key = std::env::var("DFLOW_API_KEY").unwrap_or_default();
        let input = Input {
            api_key,
            series_ticker: "KXSB".to_string(),
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
    }
}
