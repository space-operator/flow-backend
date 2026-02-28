//! Jupiter Recurring Create Order - Create a recurring DCA order
//!
//! Jupiter API: POST /recurring/v1/createOrder

use crate::prelude::*;
use serde_json::json;

pub const NAME: &str = "jupiter_recurring_create_order";
const DEFINITION: &str = flow_lib::node_definition!("jupiter/jupiter_recurring_create_order.jsonc");

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
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    pub params: JsonValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://api.jup.ag/recurring/v1/createOrder".to_string();

    let body = json!({
        "user": input.user,
        "inputMint": input.input_mint,
        "outputMint": input.output_mint,
        "params": input.params,
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
            "inputMint" => "So11111111111111111111111111111111111111112",
            "outputMint" => "So11111111111111111111111111111111111111112",
            "params" => serde_json::json!({}),
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
