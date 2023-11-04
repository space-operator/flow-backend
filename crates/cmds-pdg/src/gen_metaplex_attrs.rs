use flow_lib::{
    command::{
        builder::{BuildResult, BuilderCache, CmdBuilder},
        CommandDescription, CommandError,
    },
    Context,
};
use pdg_common::nft_metadata::{
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

inventory::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    attributes: RenderParams,
}

#[derive(Serialize, Debug)]
struct Output {
    attributes: Vec<MetaplexAttribute>,
}

async fn run(_: Context, input: Input) -> Result<Output, CommandError> {
    Ok(Output {
        attributes: NftTraits::new(&input.attributes).gen_metaplex_attrs()?,
    })
}
