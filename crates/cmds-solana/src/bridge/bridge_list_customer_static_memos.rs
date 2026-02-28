use crate::prelude::*;
use super::helper::{bridge_get, check_response};

pub const NAME: &str = "bridge_list_customer_static_memos";
const DEFINITION: &str = flow_lib::node_definition!("bridge/bridge_list_customer_static_memos.jsonc");

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
    #[serde(default)]
    pub limit: Option<u64>,
    #[serde(default)]
    pub starting_after: Option<String>,
    #[serde(default)]
    pub ending_before: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = format!("/v0/customers/{}/static_memos", input.customer_id);
    let mut req = bridge_get(&ctx, &path, &input.api_key);
    let mut query: Vec<(&str, String)> = Vec::new();
    if let Some(limit) = input.limit { query.push(("limit", limit.to_string())); }
    if let Some(ref after) = input.starting_after { query.push(("starting_after", after.clone())); }
    if let Some(ref before) = input.ending_before { query.push(("ending_before", before.clone())); }
    if !query.is_empty() { req = req.query(&query); }
    let result = check_response(req.send().await?).await?;
    Ok(Output { result })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_list_customer_static_memos() {
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
            .get(format!("https://api.sandbox.bridge.xyz/v0/customers/{customer_id}/static_memos"))
            .header("Api-Key", &api_key)
            .query(&[("limit", "2")])
            .send().await.expect("request failed");
        assert!(resp.status().is_success(), "status: {}", resp.status());
        let _body: serde_json::Value = resp.json().await.expect("json parse failed");
    }
}
