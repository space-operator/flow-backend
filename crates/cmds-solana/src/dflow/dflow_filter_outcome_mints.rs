//! DFlow Filter Outcome Mints - Filter a list of addresses to identify which are outcome mints.
//!
//! DFlow Metadata API: POST /api/v1/filter_outcome_mints

use crate::prelude::*;
use serde_json::json;

pub const NAME: &str = "dflow_filter_outcome_mints";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_filter_outcome_mints.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub addresses: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub outcome_mints: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://dev-prediction-markets-api.dflow.net/api/v1/filter_outcome_mints".to_string();

    let body = json!({
        "addresses": input.addresses,
    });

    let resp = ctx
        .http()
        .post(&url)
        .header("x-api-key", &input.api_key)
        .header("Content-Type", "application/json")
        .json(&body)
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

    let outcome_mints = response;

    Ok(Output { outcome_mints })
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
            "addresses" => serde_json::json!({}),
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
