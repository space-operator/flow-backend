use flow_lib::{
    Context,
    command::{
        CommandDescription, CommandError,
        builder::{BuildResult, BuilderCache, CmdBuilder},
    },
};
use pdg_common::nft_metadata::{
    RenderParams,
    generate::{Effect, EffectsList},
};
use serde::{Deserialize, Serialize};

const NAME: &str = "get_effect_list";

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(flow_lib::node_definition!("get_effect_list.json"))?.check_name(NAME)
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    attributes: RenderParams,
}

#[derive(Serialize, Debug)]
struct Output {
    effects: Vec<Effect>,
}

async fn run(_: Context, input: Input) -> Result<Output, CommandError> {
    Ok(Output {
        effects: EffectsList::from(input.attributes)
            .effects
            .into_iter()
            .collect(),
    })
}
