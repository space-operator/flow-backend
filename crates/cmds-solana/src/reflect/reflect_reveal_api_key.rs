use crate::prelude::*;
use super::helper::{check_response, reflect_post};

pub const NAME: &str = "reveal_api_key";
const DEFINITION: &str = flow_lib::node_definition!("reflect/reveal_api_key.jsonc");

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
    pub integration_id: String,
    pub signer: String,
    pub signature: String,
    pub timestamp: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = "/integration/api-key/reveal";
    let mut req = reflect_post(&ctx, path);
    let mut query: Vec<(&str, &str)> = Vec::new();
    if let Some(ref val) = input.cluster {
        query.push(("cluster", val.as_str()));
    }
    if !query.is_empty() {
        req = req.query(&query);
    }
    let mut body = serde_json::Map::new();
    body.insert("integrationId".into(), serde_json::Value::String(input.integration_id.clone()));
    body.insert("signer".into(), serde_json::Value::String(input.signer.clone()));
    body.insert("signature".into(), serde_json::Value::String(input.signature.clone()));
    body.insert("timestamp".into(), serde_json::json!(input.timestamp));
    req = req.json(&serde_json::Value::Object(body));
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
