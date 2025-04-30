use std::time::Duration;

use flow_lib::command::prelude::*;
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
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Output {
    pub value: Value,
}
async fn run(mut ctx: CommandContextX, input: Input) -> Result<Output, CommandError> {
    Ok(Output {
        value: ctx
            .api_input(input.timeout.map(Duration::from_secs_f64))
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
        let ctx = CommandContextX::test_context();
        build()
            .unwrap()
            .run(ctx, ValueSet::new())
            .await
            .unwrap_err();
    }
}
