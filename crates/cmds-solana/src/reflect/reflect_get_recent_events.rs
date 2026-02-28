use crate::prelude::*;
use super::helper::{check_response, reflect_get};

pub const NAME: &str = "reflect_get_recent_events";
const DEFINITION: &str = flow_lib::node_definition!("reflect/get_recent_events.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub limit: i64,
    #[serde(default)]
    pub non_zero: Option<String>,
    #[serde(default)]
    pub minimum_value: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = format!("/events/all/{}", input.limit);
    let mut req = reflect_get(&ctx, &path);
    let mut query: Vec<(&str, &str)> = Vec::new();
    if let Some(ref val) = input.non_zero {
        query.push(("nonZero", val.as_str()));
    }
    if let Some(ref val) = input.minimum_value {
        query.push(("minimumValue", val.as_str()));
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
