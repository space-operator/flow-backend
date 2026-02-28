//! DFlow Verify Address - Check if a wallet address has been KYC verified.
//!
//! DFlow Proof API: GET /verify/{address}

use crate::prelude::*;

pub const NAME: &str = "dflow_verify_address";
const DEFINITION: &str = flow_lib::node_definition!("dflow/dflow_verify_address.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub verified: bool,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = format!("https://proof.dflow.net/verify/{}", input.address);

    let query: Vec<(&str, String)> = Vec::new();


    let resp = ctx
        .http()
        .get(&url)
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

    let verified = response.get("verified").and_then(|v| v.as_bool()).unwrap_or(false);

    Ok(Output { verified })
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
            "address" => "11111111111111111111111111111112",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_deserialize_response() {
        let json_str = include_str!("fixtures/verify.json");
        let _parsed: crate::dflow::response_types::VerifyResponse = serde_json::from_str(json_str)
            .expect("Failed to deserialize verify.json");
    }

    #[tokio::test]
    #[ignore] // Hits live dev endpoint; run with: cargo test -- --ignored
    async fn test_run_verify_address() {
        let input = Input {
            address: "11111111111111111111111111111112".to_string(),
        };
        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run() failed: {:?}", result.err());
    }
}
