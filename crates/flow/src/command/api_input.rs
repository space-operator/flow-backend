use crate::command::prelude::*;

const NAME: &str = "name";

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/api_input.json");
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Input {}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Output {
    value: Value,
}

async fn run(mut ctx: CommandContextX, _: Input) -> Result<Output, CommandError> {
    Ok(Output {
        value: ctx.api_input().await?.value,
    })
}
