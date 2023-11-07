use flow_lib::{
    command::{
        builder::{BuildResult, BuilderCache, CmdBuilder},
        CommandDescription, CommandError,
    },
    Context, Value,
};
use pdg_common::nft_metadata::RenderParams;
use serde::{Deserialize, Serialize};

const PARSE_PDG_ATTRS: &str = "parse_pdg_attrs";

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(flow_lib::node_definition!("parse_pdg_attrs.json"))?
            .check_name(PARSE_PDG_ATTRS)
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(PARSE_PDG_ATTRS, |_| build()));

const fn bool_true() -> bool {
    true
}

#[derive(Deserialize, Debug)]
struct Input {
    attributes: Value,
    #[serde(default = "bool_true")]
    check_human_readable: bool,
}

#[derive(Serialize, Debug)]
struct Output {
    attributes: RenderParams,
}

async fn run(_: Context, input: Input) -> Result<Output, CommandError> {
    Ok(Output {
        attributes: RenderParams::from_pdg_metadata(
            &mut input.attributes.into(),
            input.check_human_readable,
        )?,
    })
}
