use crate::prelude::*;
use super::helper::{check_response, reflect_post_auth};

pub const NAME: &str = "whitelist_address";
const DEFINITION: &str = flow_lib::node_definition!("reflect/whitelist_address.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    #[serde(default)]
    pub cluster: Option<String>,
    pub signer: String,
    #[serde(default)]
    pub user: Option<String>,
    pub fee_payer: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = "/integration/whitelist";
    let mut req = reflect_post_auth(&ctx, path, &input.api_key);
    let mut query: Vec<(&str, &str)> = Vec::new();
    if let Some(ref val) = input.cluster {
        query.push(("cluster", val.as_str()));
    }
    if !query.is_empty() {
        req = req.query(&query);
    }
    let mut body = serde_json::Map::new();
    body.insert("signer".into(), serde_json::Value::String(input.signer.clone()));
    if let Some(ref val) = input.user {
        body.insert("user".into(), serde_json::Value::String(val.clone()));
    }
    body.insert("feePayer".into(), serde_json::Value::String(input.fee_payer.clone()));
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
