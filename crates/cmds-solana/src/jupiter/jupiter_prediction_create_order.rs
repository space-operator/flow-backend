//! Jupiter Prediction Create Order - Create prediction order
//!
//! Jupiter API: POST /prediction/v1/orders

use crate::prelude::*;
use serde_json::json;

pub const NAME: &str = "jupiter_prediction_create_order";
const DEFINITION: &str = flow_lib::node_definition!("jupiter/jupiter_prediction_create_order.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub user: String,
    #[serde(rename = "marketId")]
    pub market_id: String,
    pub side: String,
    #[serde(rename = "outcomeId")]
    pub outcome_id: String,
    pub amount: String,
    pub price: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://api.jup.ag/prediction/v1/orders".to_string();

    let body = json!({
        "user": input.user,
        "marketId": input.market_id,
        "side": input.side,
        "outcomeId": input.outcome_id,
        "amount": input.amount,
        "price": input.price,
    });

    let req = ctx
        .http()
        .post(&url)
        .header("x-api-key", &input.api_key)
        .header("Content-Type", "application/json")
        .json(&body);

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
            "user" => "test-value",
            "marketId" => "test-id-123",
            "side" => "test-value",
            "outcomeId" => "test-id-123",
            "amount" => "1000000",
            "price" => "test-value",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
