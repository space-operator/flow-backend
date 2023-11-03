use flow_lib::{
    command::{
        builder::{BuildResult, BuilderCache, CmdBuilder},
        CommandDescription, CommandError,
    },
    Context,
};
use pdg_common::nft_metadata::{
    generate::{Effect, EffectsList},
    metaplex::{MetaplexAttribute, NftTraits},
    RenderParams,
};
use serde::{Deserialize, Serialize};

const NAME: &str = "gen_metaplex_attrs";

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(flow_lib::node_definition!("gen_metaplex_attrs.json"))?.check_name(NAME)
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    attributes: RenderParams,
    effects: Vec<Effect>,
}

#[derive(Serialize, Debug)]
struct Output {
    attributes: Vec<MetaplexAttribute>,
}

async fn run(_: Context, input: Input) -> Result<Output, CommandError> {
    let traits = NftTraits::new(&input.attributes, &EffectsList::from(input.effects));
    Ok(Output {
        attributes: traits.gen_metaplex_attrs()?,
    })
}
