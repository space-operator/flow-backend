use mpl_core::{instructions::UpdateV1Builder, types::UpdateAuthority};

use crate::prelude::*;

// Command Name
const NAME: &str = "update_core_v1";

const DEFINITION: &str = flow_lib::node_definition!("nft/core/mpl_core_update_asset.jsonc");

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
    pub asset: Wallet,
    #[serde(default, with = "value::pubkey::opt")]
    pub collection: Option<Pubkey>,
    pub new_name: Option<String>,
    pub new_uri: Option<String>,
    pub new_update_authority: Option<UpdateAuthority>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut builder = UpdateV1Builder::new();

    let builder = builder
        .asset(input.asset.pubkey())
        .payer(input.fee_payer.pubkey());

    let builder = match input.collection {
        Some(collection) => builder.collection(Some(collection)),
        _ => builder,
    };

    let builder = match input.new_name {
        Some(new_name) => builder.new_name(new_name),
        _ => builder,
    };

    let builder = match input.new_uri {
        Some(new_uri) => builder.new_uri(new_uri),
        _ => builder,
    };

    let builder = match input.new_update_authority {
        Some(new_update_authority) => builder.new_update_authority(new_update_authority),
        _ => builder,
    };

    let ins = builder.instruction();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
        instructions: [ins].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
