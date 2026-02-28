//! DFlow Get Forecast History - Get event forecast percentile history.
//!
//! DFlow Metadata API: GET /api/v1/event/{series_ticker}/{event_id}/forecast_percentile_history

use crate::prelude::*;

pub const NAME: &str = "dflow_get_forecast_history";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_get_forecast_history.jsonc");

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
    pub event_id: String,
    pub percentiles: String,
    pub start_ts: u64,
    pub end_ts: u64,
    pub period_interval: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub forecast_history: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = format!("https://dev-prediction-markets-api.dflow.net/api/v1/event/{}/{}/forecast_percentile_history", input.series_ticker, input.event_id);

    let mut query: Vec<(&str, String)> = Vec::new();
    query.push(("percentiles", input.percentiles.to_string()));
    query.push(("start_ts", input.start_ts.to_string()));
    query.push(("end_ts", input.end_ts.to_string()));
    query.push(("period_interval", input.period_interval.to_string()));

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

    let forecast_history = response;

    Ok(Output { forecast_history })
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
            "event_id" => "KXSB-26",
            "percentiles" => "test-value",
            "start_ts" => 1_u64,
            "end_ts" => 1_u64,
            "period_interval" => 1_u64,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
