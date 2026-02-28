use crate::prelude::*;
use super::helper::{bridge_get, check_response};

pub const NAME: &str = "bridge_get_customer";
const DEFINITION: &str = flow_lib::node_definition!("bridge/bridge_get_customer.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub customer_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = format!("/v0/customers/{}", input.customer_id);
    let result = check_response(
        bridge_get(&ctx, &path, &input.api_key)
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
            format!("{}/tests/fixtures/customer.json", env!("CARGO_MANIFEST_DIR"))
        ).unwrap();
        let _parsed: crate::bridge::response_types::Customer = serde_json::from_str(&json).unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_get_customer() {
        let api_key = match std::env::var("BRIDGE_API_KEY") {
            Ok(k) => k,
            Err(_) => { eprintln!("BRIDGE_API_KEY not set, skipping"); return; }
        };
        let client = reqwest::Client::new();
        // Fetch a customer ID first
        let resp = client
            .get("https://api.sandbox.bridge.xyz/v0/customers")
            .header("Api-Key", &api_key)
            .query(&[("limit", "1")])
            .send().await.expect("customer list failed");
        let body: serde_json::Value = resp.json().await.expect("json");
        let customers = body["data"].as_array().unwrap();
        if customers.is_empty() { eprintln!("No customers in sandbox, skipping"); return; }
        let customer_id = customers[0]["id"].as_str().unwrap();
        let resp = client
            .get(format!("https://api.sandbox.bridge.xyz/v0/customers/{customer_id}"))
            .header("Api-Key", &api_key)
            .send().await.expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _body: serde_json::Value = resp.json().await.expect("json parse failed");
    }
}
