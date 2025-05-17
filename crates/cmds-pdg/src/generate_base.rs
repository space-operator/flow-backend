use flow_lib::command::prelude::*;
use pdg_common::nft_metadata::RenderParams;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};

const NAME: &str = "generate_base";

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(flow_lib::node_definition!("generate_base.json"))?.check_name(NAME)
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    #[serde(default)]
    seed: Option<u64>,
    #[serde(default)]
    defaults: flow_lib::value::Map,
}

#[derive(Serialize, Debug)]
struct Output {
    attributes: RenderParams,
}

async fn run(_: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut rng = match input.seed {
        Some(seed) => ChaCha20Rng::seed_from_u64(seed),
        None => ChaCha20Rng::from_entropy(),
    };

    let attributes = RenderParams::generate_base(&mut rng);

    let mut map = flow_lib::value::to_map(&attributes)?;
    map.extend(input.defaults.into_iter());
    let attributes = flow_lib::value::from_map(map)?;

    Ok(Output { attributes })
}
