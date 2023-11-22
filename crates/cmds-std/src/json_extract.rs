use flow_lib::{command::prelude::*, UserId};

const NAME: &str = "flow_run_info";

fn build() -> BuildResult {
    Ok(
        CmdBuilder::new(flow_lib::node_definition!("flow_run_info.json"))?
            .check_name(NAME)?
            .build(run),
    )
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {}

#[derive(Serialize, Debug)]
struct Output {
    flow_owner: UserId,
    started_by: UserId,
}

async fn run(ctx: Context, _: Input) -> Result<Output, CommandError> {
    Ok(Output {
        flow_owner: ctx.flow_owner.id,
        started_by: ctx.started_by.id,
    })
}
