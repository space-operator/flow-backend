//! DFlow Init Prediction Market - Initialize a prediction market and get transaction to sign.
//!
//! DFlow Trading API: GET /prediction-market-init

use crate::prelude::*;

pub const NAME: &str = "dflow_init_prediction_market";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_init_prediction_market.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub payer: String,
    pub outcome_mint: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub transaction: String,
    pub last_valid_block_height: u64,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://quote-api.dflow.net/prediction-market-init".to_string();

    let query: Vec<(&str, String)> = vec![
        ("payer", input.payer.to_string()),
        ("outcome_mint", input.outcome_mint.to_string()),
    ];

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

    let transaction = response.get("transaction").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let last_valid_block_height = response.get("last_valid_block_height").and_then(|v| v.as_u64()).unwrap_or(0);

    Ok(Output { transaction, last_valid_block_height })
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
            "payer" => "11111111111111111111111111111112",
            "outcome_mint" => "So11111111111111111111111111111111111111112",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
