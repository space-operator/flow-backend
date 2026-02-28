//! Jupiter Portfolio Staked Jup - Get staked JUP info for address
//!
//! Jupiter API: GET /portfolio/v1/staked-jup/{address}

use crate::prelude::*;

pub const NAME: &str = "jupiter_portfolio_staked_jup";
const DEFINITION: &str = flow_lib::node_definition!("jupiter/jupiter_portfolio_staked_jup.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = format!("https://api.jup.ag/portfolio/v1/staked-jup/{}", input.address);

    let req = ctx
        .http()
        .get(&url)
        .header("x-api-key", &input.api_key);

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
            "address" => "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[tokio::test]
    #[ignore = "requires JUPITER_API_KEY"]
    async fn test_run_portfolio_staked_jup() {
        let api_key = match std::env::var("JUPITER_API_KEY") {
            Ok(k) => k,
            Err(_) => { eprintln!("JUPITER_API_KEY not set, skipping"); return; }
        };
        let input = Input {
            api_key,
            address: "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string(),
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
    }
}
