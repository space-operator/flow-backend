//! Jupiter Tokens Category - Get top tokens by category and interval
//!
//! Jupiter API: GET /tokens/v2/{category}/{interval}

use crate::prelude::*;

pub const NAME: &str = "jupiter_tokens_category";
const DEFINITION: &str = flow_lib::node_definition!("jupiter/jupiter_tokens_category.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub category: String,
    pub interval: String,
    #[serde(default)]
    pub limit: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = format!("https://api.jup.ag/tokens/v2/{}/{}", input.category, input.interval);

    let mut query: Vec<(&str, String)> = Vec::new();
    if let Some(ref v) = input.limit {
        query.push(("limit", v.clone()));
    }

    let req = ctx
        .http()
        .get(&url)
        .header("x-api-key", &input.api_key)
        .query(&query);

    let resp = req.send().await?;

    if !resp.status().is_success() {
        return Err(CommandError::msg(format!(
            "Jupiter API error: {} {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }

    let response: JsonValue = resp.json().await?;

    Ok(Output { result: response })
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
            "category" => "test-value",
            "interval" => "test-value",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[tokio::test]
    #[ignore = "requires JUPITER_API_KEY"]
    async fn test_run_tokens_category() {
        let api_key = match std::env::var("JUPITER_API_KEY") {
            Ok(k) => k,
            Err(_) => { eprintln!("JUPITER_API_KEY not set, skipping"); return; }
        };
        let input = Input {
            api_key,
            category: "toptraded".to_string(),
            interval: "24h".to_string(),
            limit: None,
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
    }
}
