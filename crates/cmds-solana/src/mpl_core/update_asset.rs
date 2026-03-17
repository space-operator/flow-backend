use mpl_core::{instructions::UpdateV1Builder, types::UpdateAuthority};

use crate::prelude::*;

// Command Name
const NAME: &str = "update_core_v1";

const DEFINITION: &str = flow_lib::node_definition!("mpl_core/mpl_core_update_asset.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub asset: Pubkey,
    pub authority: Option<Wallet>,
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

    builder
        .asset(input.asset)
        .payer(input.fee_payer.pubkey());

    if let Some(collection) = input.collection {
        builder.collection(Some(collection));
    }
    if let Some(ref authority) = input.authority {
        builder.authority(Some(authority.pubkey()));
    }
    if let Some(new_name) = input.new_name {
        builder.new_name(new_name);
    }
    if let Some(new_uri) = input.new_uri {
        builder.new_uri(new_uri);
    }
    if let Some(new_update_authority) = input.new_update_authority {
        builder.new_update_authority(new_update_authority);
    }

    let ins = builder.instruction();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer]
            .into_iter()
            .chain(input.authority)
            .collect(),
        instructions: [ins].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
