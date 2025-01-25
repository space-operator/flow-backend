use flow_lib::{
    command::{
        builder::{BuildResult, BuilderCache, CmdBuilder},
        CommandDescription, CommandError,
    },
    Context,
};
use pdg_common::nft_metadata::{generate::Effect, RenderParams};
use serde::{Deserialize, Serialize};

const NAME: &str = "update_render_params";

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(flow_lib::node_definition!("update_render_params.json"))?.check_name(NAME)
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    params: RenderParams,
    effect: Effect,
}

#[derive(Serialize, Debug)]
struct Output {
    params: RenderParams,
}

async fn run(_: Context, mut input: Input) -> Result<Output, CommandError> {
    input.params.add_effect(input.effect);
    Ok(Output {
        params: input.params,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
