use crate::prelude::*;
use super::helper::{bridge_get, check_response};

pub const NAME: &str = "bridge_get_liquidation_drains";
const DEFINITION: &str = flow_lib::node_definition!("bridge/bridge_get_liquidation_drains.jsonc");

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
    pub liquidation_address_id: String,
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
    let path = format!("/v0/customers/{}/liquidation_addresses/{}/drains", input.customer_id, input.liquidation_address_id);
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
}
