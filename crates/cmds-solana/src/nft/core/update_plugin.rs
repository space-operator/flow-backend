use mpl_core::{instructions::UpdatePluginV1Builder, types::Plugin};

use crate::prelude::*;

// Command Name
const NAME: &str = "mpl_core_update_plugin";

const DEFINITION: &str = flow_lib::node_definition!("nft/core/mpl_core_update_plugin.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    #[serde(with = "value::pubkey")]
    pub asset: Pubkey,
    pub update_authority: Option<Wallet>,
    #[serde(default, with = "value::pubkey::opt")]
    pub collection: Option<Pubkey>,
    pub plugin: Plugin,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContextX, input: Input) -> Result<Output, CommandError> {
    let plugin: Plugin = input.plugin;

    let mut builder = UpdatePluginV1Builder::new();

    let builder = builder
        .asset(input.asset)
        .payer(input.fee_payer.pubkey())
        .collection(input.collection)
        .plugin(plugin);

    let builder = match input.update_authority {
        Some(ref update_authority) => builder.authority(Some(update_authority.pubkey())),
        _ => builder,
    };

    let ins = builder.instruction();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer]
            .into_iter()
            .chain(input.update_authority)
            .collect(),
        instructions: [ins].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
