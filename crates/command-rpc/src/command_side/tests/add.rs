use flow_lib::command::prelude::*;
const NAME: &str = "add";
flow_lib::submit!(CommandDescription::new(NAME, |_| build()));
pub(crate) fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("command_side/tests/add.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    #[serde_as(as = "AsDecimal")]
    a: Decimal,
    #[serde_as(as = "AsDecimal")]
    b: Decimal,
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Output {
    #[serde_as(as = "AsDecimal")]
    pub c: Decimal,
}
async fn run(_: CommandContext, Input { a, b }: Input) -> Result<Output, CommandError> {
    Ok(Output { c: a + b })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_build() {
        build().unwrap();
    }
}
