use flow_lib::{command::prelude::*, SolanaNet};

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
    flow_owner: String,
    started_by: String,
    solana_net: SolanaNet,
}

async fn run(ctx: Context, _: Input) -> Result<Output, CommandError> {
    Ok(Output {
        flow_owner: ctx.flow_owner.id.to_string(),
        started_by: ctx.started_by.id.to_string(),
        solana_net: ctx.cfg.solana_client.cluster,
    })
}
