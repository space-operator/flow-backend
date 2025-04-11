use flow_lib::command::prelude::*;
use std::time::Duration;
use tokio::time;

pub const NAME: &str = "wait";

const DEFINITION: &str = flow_lib::node_definition!("wait.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    value: Value,
    wait_for: Value,
    duration_ms: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {}

async fn run(_ctx: CommandContextX, input: Input) -> Result<Output, CommandError> {
    if let Some(duration) = input.duration_ms {
        time::sleep(Duration::from_millis(duration)).await;
    }

    Ok(Output {})
}
