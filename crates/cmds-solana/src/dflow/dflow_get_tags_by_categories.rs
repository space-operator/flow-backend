//! DFlow Get Tags By Categories - Get prediction market tags organized by category.
//!
//! DFlow Metadata API: GET /api/v1/tags_by_categories

use crate::prelude::*;

pub const NAME: &str = "dflow_get_tags_by_categories";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_get_tags_by_categories.jsonc");

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
    pub tags_by_categories: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://dev-prediction-markets-api.dflow.net/api/v1/tags_by_categories".to_string();

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

    let tags_by_categories = response;

    Ok(Output { tags_by_categories })
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
    async fn test_run_get_tags_by_categories() {
        let api_key = std::env::var("DFLOW_API_KEY").unwrap_or_default();
        let input = Input {
            api_key,
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
    }
}
