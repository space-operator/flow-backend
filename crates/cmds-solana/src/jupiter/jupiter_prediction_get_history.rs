//! Jupiter Prediction Get History - Get trading history
//!
//! Jupiter API: GET /prediction/v1/history

use crate::prelude::*;

pub const NAME: &str = "jupiter_prediction_get_history";
const DEFINITION: &str = flow_lib::node_definition!("jupiter/jupiter_prediction_get_history.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    #[serde(rename = "ownerPubkey")]
    pub owner_pubkey: String,
    #[serde(default)]
    pub page: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://api.jup.ag/prediction/v1/history".to_string();

    let mut query: Vec<(&str, String)> = Vec::new();
    query.push(("ownerPubkey", input.owner_pubkey.clone()));
    if let Some(ref v) = input.page {
        query.push(("page", v.clone()));
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
            "ownerPubkey" => "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[tokio::test]
    #[ignore = "requires JUPITER_API_KEY"]
    async fn test_run_prediction_get_history() {
        let api_key = match std::env::var("JUPITER_API_KEY") {
            Ok(k) => k,
            Err(_) => { eprintln!("JUPITER_API_KEY not set, skipping"); return; }
        };
        let input = Input {
            api_key,
            owner_pubkey: "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string(),
            page: None,
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
    }
}
