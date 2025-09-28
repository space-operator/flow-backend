use flow_lib::{command::prelude::*, context::execute};
const NAME: &str = "error_node";
flow_lib::submit!(CommandDescription::new(NAME, |_| build()));
pub fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("error_node.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    x: Option<u64>,
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Output {
    pub output: u64,
}
async fn run(_: CommandContext, input: Input) -> Result<Output, CommandError> {
    tracing::info!("input: {:?}", input);
    Err(match input.x {
        Some(0) => execute::Error::Collected.into(),
        _ => CommandError::msg("unimplemented"),
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
