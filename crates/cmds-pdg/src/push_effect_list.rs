use flow_lib::{
    Context,
    command::{
        CommandDescription, CommandError,
        builder::{BuildResult, BuilderCache, CmdBuilder},
    },
};
use pdg_common::nft_metadata::generate::{Effect, EffectsList};
use serde::{Deserialize, Serialize};

const NAME: &str = "push_effect_list";

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(flow_lib::node_definition!("push_effect_list.json"))?.check_name(NAME)
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    effects: Vec<Effect>,
    element: Effect,
}

#[derive(Serialize, Debug)]
struct Output {
    effects: Vec<Effect>,
}

async fn run(_: Context, input: Input) -> Result<Output, CommandError> {
    let mut e = EffectsList::from(input.effects);
    e.push(input.element);
    Ok(Output {
        effects: e.effects.into_iter().collect(),
    })
}
