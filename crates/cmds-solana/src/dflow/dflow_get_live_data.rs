//! DFlow Get Live Data - Get live data by milestone IDs.
//!
//! DFlow Metadata API: GET /api/v1/live_data

use crate::prelude::*;

pub const NAME: &str = "dflow_get_live_data";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_get_live_data.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub milestone_ids: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub live_data: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://dev-prediction-markets-api.dflow.net/api/v1/live_data".to_string();

    let mut query: Vec<(&str, String)> = Vec::new();
    query.push(("milestone_ids", input.milestone_ids.to_string()));

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

    let live_data = response;

    Ok(Output { live_data })
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
            "milestone_ids" => serde_json::json!({}),
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
