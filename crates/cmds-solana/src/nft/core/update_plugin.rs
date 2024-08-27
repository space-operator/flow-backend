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
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
    #[serde(with = "value::pubkey")]
    pub asset: Pubkey,
    #[serde(with = "value::keypair::opt")]
    pub update_authority: Option<Keypair>,
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

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let plugin: Plugin = input.plugin;

    let mut builder = UpdatePluginV1Builder::new();

    let builder = builder
        .asset(input.asset)
        .payer(input.fee_payer.pubkey())
        .collection(input.collection.map(Into::into))
        .plugin(plugin);

    let builder = if let Some(ref update_authority) = input.update_authority {
        builder.authority(Some(update_authority.pubkey()))
    } else {
        builder
    };

    let ins = builder.instruction();

    let mut signers = vec![input.fee_payer.clone_keypair()];

    if let Some(authority) = input.update_authority.as_ref() {
        signers.push(authority.clone_keypair());
    }

    let ins = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers,
        instructions: [ins].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
