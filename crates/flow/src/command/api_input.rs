use std::time::Duration;

use flow_lib::command::prelude::*;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
const NAME: &str = "api_input";
flow_lib::submit!(CommandDescription::new(NAME, |_| build()));
fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("command/api_input.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    timeout: Option<f64>,
    webhook_url: Option<String>,
    webhook_headers: Option<Vec<(String, String)>>,
    extra: Option<serde_json::Map<String, serde_json::Value>>,
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Output {
    pub value: Value,
}
async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let headers = match input.webhook_headers {
        Some(vec) => {
            let map = vec
                .into_iter()
                .map(|(k, v)| {
                    Ok::<_, CommandError>((k.parse::<HeaderName>()?, v.parse::<HeaderValue>()?))
                })
                .collect::<Result<HeaderMap, _>>()?;
            Some(map)
        }
        None => None,
    };
    Ok(Output {
        value: ctx
            .api_input(
                input.timeout.map(Duration::from_secs_f64),
                input.webhook_url,
                headers,
                input.extra,
            )
            .await?
            .value,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_build() {
        build().unwrap();
    }
    #[tokio::test]
    async fn test_run() {
        let ctx = CommandContext::test_context();
        build()
            .unwrap()
            .run(ctx, ValueSet::new())
            .await
            .unwrap_err();
    }
}
