use crate::prelude::*;
use super::helper::{check_response, reflect_get};

pub const NAME: &str = "get_historical_stats";
const DEFINITION: &str = flow_lib::node_definition!("reflect/get_historical_stats.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub days: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = "/stats/historical";
    let mut req = reflect_get(&ctx, path);
    let mut query: Vec<(&str, &str)> = Vec::new();
    let days_str = input.days.to_string();
    query.push(("days", &days_str));
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
