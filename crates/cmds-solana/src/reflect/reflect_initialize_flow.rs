use crate::prelude::*;
use super::helper::{check_response, reflect_post};

pub const NAME: &str = "reflect_initialize_flow";
const DEFINITION: &str = flow_lib::node_definition!("reflect/initialize_flow.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(default)]
    pub cluster: Option<String>,
    pub signer: String,
    pub authority: String,
    pub stablecoin: i64,
    pub fee_bps: i64,
    pub fee_payer: String,
    pub metadata_name: String,
    pub metadata_symbol: String,
    pub metadata_description: String,
    pub metadata_uri: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = "/integration/initialize/flow";
    let mut req = reflect_post(&ctx, path);
    let mut query: Vec<(&str, &str)> = Vec::new();
    if let Some(ref val) = input.cluster {
        query.push(("cluster", val.as_str()));
    }
    if !query.is_empty() {
        req = req.query(&query);
    }
    let body = serde_json::json!({
        "signer": input.signer,
        "authority": input.authority,
        "stablecoin": input.stablecoin,
        "feeBps": input.fee_bps,
        "feePayer": input.fee_payer,
        "metadata": {
            "name": input.metadata_name,
            "symbol": input.metadata_symbol,
            "description": input.metadata_description,
            "uri": input.metadata_uri,
        }
    });
    req = req.json(&body);
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
