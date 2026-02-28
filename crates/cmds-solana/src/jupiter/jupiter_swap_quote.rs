//! Jupiter Swap Quote - Get a swap quote for POST /swap
//!
//! Jupiter API: GET /swap/v1/quote

use crate::prelude::*;

pub const NAME: &str = "jupiter_swap_quote";
const DEFINITION: &str = flow_lib::node_definition!("jupiter/jupiter_swap_quote.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    pub amount: String,
    #[serde(default, rename = "slippageBps")]
    pub slippage_bps: Option<String>,
    #[serde(default, rename = "swapMode")]
    pub swap_mode: Option<String>,
    #[serde(default)]
    pub dexes: Option<String>,
    #[serde(default, rename = "excludeDexes")]
    pub exclude_dexes: Option<String>,
    #[serde(default, rename = "platformFeeBps")]
    pub platform_fee_bps: Option<String>,
    #[serde(default, rename = "maxAccounts")]
    pub max_accounts: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://api.jup.ag/swap/v1/quote".to_string();

    let mut query: Vec<(&str, String)> = Vec::new();
    query.push(("inputMint", input.input_mint.clone()));
    query.push(("outputMint", input.output_mint.clone()));
    query.push(("amount", input.amount.clone()));
    if let Some(ref v) = input.slippage_bps {
        query.push(("slippageBps", v.clone()));
    }
    if let Some(ref v) = input.swap_mode {
        query.push(("swapMode", v.clone()));
    }
    if let Some(ref v) = input.dexes {
        query.push(("dexes", v.clone()));
    }
    if let Some(ref v) = input.exclude_dexes {
        query.push(("excludeDexes", v.clone()));
    }
    if let Some(ref v) = input.platform_fee_bps {
        query.push(("platformFeeBps", v.clone()));
    }
    if let Some(ref v) = input.max_accounts {
        query.push(("maxAccounts", v.clone()));
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
            "inputMint" => "So11111111111111111111111111111111111111112",
            "outputMint" => "So11111111111111111111111111111111111111112",
            "amount" => "1000000",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[tokio::test]
    #[ignore = "requires JUPITER_API_KEY"]
    async fn test_run_swap_quote() {
        let api_key = match std::env::var("JUPITER_API_KEY") {
            Ok(k) => k,
            Err(_) => { eprintln!("JUPITER_API_KEY not set, skipping"); return; }
        };
        let input = Input {
            api_key,
            input_mint: "So11111111111111111111111111111111111111112".to_string(),
            output_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            amount: "100000000".to_string(),
            slippage_bps: None,
            swap_mode: None,
            dexes: None,
            exclude_dexes: None,
            platform_fee_bps: None,
            max_accounts: None,
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
    }
}
