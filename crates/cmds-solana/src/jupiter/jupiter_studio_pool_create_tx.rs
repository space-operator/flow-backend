//! Jupiter Studio Pool Create Tx - Create DBC pool transaction
//!
//! Jupiter API: POST /studio/v1/dbc-pool/create-tx

use crate::prelude::*;
use serde_json::json;

pub const NAME: &str = "jupiter_studio_pool_create_tx";
const DEFINITION: &str = flow_lib::node_definition!("jupiter/jupiter_studio_pool_create_tx.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    #[serde(rename = "buildCurveByMarketCapParam")]
    pub build_curve_by_market_cap_param: JsonValue,
    #[serde(rename = "tokenName")]
    pub token_name: String,
    #[serde(rename = "tokenSymbol")]
    pub token_symbol: String,
    #[serde(default, rename = "tokenImageContentType")]
    pub token_image_content_type: Option<String>,
    pub creator: String,
    #[serde(default, rename = "antiSniping")]
    pub anti_sniping: Option<bool>,
    #[serde(default)]
    pub fee: Option<JsonValue>,
    #[serde(default, rename = "isLpLocked")]
    pub is_lp_locked: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://api.jup.ag/studio/v1/dbc-pool/create-tx".to_string();

    let mut body = json!({
        "buildCurveByMarketCapParam": input.build_curve_by_market_cap_param,
        "tokenName": input.token_name,
        "tokenSymbol": input.token_symbol,
        "creator": input.creator,
    });
    if let Some(v) = input.token_image_content_type {
        body["tokenImageContentType"] = json!(v);
    }
    if let Some(v) = input.anti_sniping {
        body["antiSniping"] = json!(v);
    }
    if let Some(v) = input.fee {
        body["fee"] = v;
    }
    if let Some(v) = input.is_lp_locked {
        body["isLpLocked"] = json!(v);
    }

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
            "buildCurveByMarketCapParam" => serde_json::json!({}),
            "tokenName" => "test-value",
            "tokenSymbol" => "test-value",
            "creator" => "test-value",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
