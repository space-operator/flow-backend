use crate::prelude::*;
use super::helper::{bridge_get, check_response};

pub const NAME: &str = "bridge_list_countries";
const DEFINITION: &str = flow_lib::node_definition!("bridge/bridge_list_countries.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = "/v0/lists/countries";
    let result = check_response(
        bridge_get(&ctx, path, &input.api_key)
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
            format!("{}/tests/fixtures/countries.json", env!("CARGO_MANIFEST_DIR"))
        ).unwrap();
        let _parsed: crate::bridge::response_types::PaginatedList<crate::bridge::response_types::Country> = serde_json::from_str(&json).unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_list_countries() {
        let api_key = match std::env::var("BRIDGE_API_KEY") {
            Ok(k) => k,
            Err(_) => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        let resp = client
            .get("https://api.sandbox.bridge.xyz/v0/lists/countries")
            .header("Api-Key", &api_key)
            .send().await.expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _body: serde_json::Value = resp.json().await.expect("json parse failed");
    }
}
