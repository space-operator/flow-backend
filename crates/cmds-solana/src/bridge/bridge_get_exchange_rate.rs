use crate::prelude::*;
use super::helper::{bridge_get, check_response};

pub const NAME: &str = "bridge_get_exchange_rate";
const DEFINITION: &str = flow_lib::node_definition!("bridge/bridge_get_exchange_rate.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = "/v0/exchange_rates";
    let result = check_response(
        bridge_get(&ctx, path, &input.api_key)
            .query(&[("from", &input.from), ("to", &input.to)])
            .send()
            .await?,
    )
    .await?;
    Ok(Output { result })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_deserialize_response() {
        let json = std::fs::read_to_string(
            format!("{}/tests/fixtures/exchange_rate.json", env!("CARGO_MANIFEST_DIR"))
        ).unwrap();
        let _parsed: crate::bridge::response_types::ExchangeRate = serde_json::from_str(&json).unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_get_exchange_rate() {
        let api_key = match std::env::var("BRIDGE_API_KEY") {
            Ok(k) => k,
            Err(_) => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get("https://api.sandbox.bridge.xyz/v0/exchange_rates")
            .header("Api-Key", &api_key)
            .query(&[("from", "usd"), ("to", "eur")])
            .send().await.expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _body: serde_json::Value = resp.json().await.expect("json parse failed");
    }
}
