use mpl_core::instructions::{CreateV1Builder, CreateV2Builder};

use crate::prelude::*;

// Command Name
const NAME: &str = "create_core_v2";

const DEFINITION: &str = flow_lib::node_definition!("nft/core/mpl_core_create_asset.json");

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
    #[serde(with = "value::keypair")]
    pub asset: Keypair,
    #[serde(with = "value::keypair::opt")]
    pub authority: Option<Keypair>,
    pub name: String,
    pub uri: String,
    #[serde(default, with = "value::pubkey::opt")]
    pub collection: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let mut builder = CreateV1Builder::new();

    let builder = builder
        .asset(input.asset.pubkey())
        .payer(input.fee_payer.pubkey())
        .name(input.name)
        .uri(input.uri);

    let builder = if let Some(ref authority) = input.authority {
        builder.authority(Some(authority.pubkey()))
    } else {
        builder.authority(None)
    };

    let builder = if let Some(collection) = input.collection {
        builder.collection(Some(collection))
    } else {
        builder
    };

    let ins = builder.instruction();

    let mut signers = vec![input.fee_payer.clone_keypair(), input.asset.clone_keypair()];

    if let Some(authority) = input.authority.as_ref() {
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
