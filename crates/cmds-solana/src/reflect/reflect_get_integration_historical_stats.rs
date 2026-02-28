use crate::prelude::*;
use super::helper::{check_response, reflect_get_auth};

pub const NAME: &str = "get_integration_historical_stats";
const DEFINITION: &str = flow_lib::node_definition!("reflect/get_integration_historical_stats.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub integration_id: String,
    #[serde(default)]
    pub period: Option<String>,
    #[serde(default)]
    pub cluster: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = format!("/integration/{}/stats/historical", input.integration_id);
    let mut req = reflect_get_auth(&ctx, &path, &input.api_key);
    let mut query: Vec<(&str, &str)> = Vec::new();
    if let Some(ref val) = input.period {
        query.push(("period", val.as_str()));
    }
    if let Some(ref val) = input.cluster {
        query.push(("cluster", val.as_str()));
    }
    if !query.is_empty() {
        req = req.query(&query);
    }
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
