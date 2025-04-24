use flow_lib::command::prelude::*;
use pdg_common::nft_metadata::RenderParams;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

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
    #[serde(default)]
    defaults: HashMap<String, JsonValue>,
}

#[derive(Serialize, Debug)]
struct Output {
    attributes: RenderParams,
}

async fn run(_: CommandContextX, input: Input) -> Result<Output, CommandError> {
    Ok(Output {
        attributes: RenderParams::from_pdg_metadata(
            &mut input.attributes.into(),
            input.check_human_readable,
            &input.defaults,
        )?,
    })
}
