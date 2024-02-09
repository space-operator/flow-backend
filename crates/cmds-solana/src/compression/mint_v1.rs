use crate::prelude::*;
use mpl_bubblegum::instructions::MintV1Builder;
use solana_sdk::pubkey::Pubkey;

use super::MetadataBubblegum;

// Command Name
const NAME: &str = "mint_compressed_NFT";

const DEFINITION: &str = flow_lib::node_definition!("compression/mint_compressed_NFT.json");

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
    pub payer: Keypair,
    #[serde(with = "value::keypair")]
    pub creator_or_delegate: Keypair,
    #[serde(with = "value::pubkey")]
    pub tree_config: Pubkey,
    #[serde(with = "value::pubkey")]
    pub merkle_tree: Pubkey,
    #[serde(with = "value::pubkey")]
    pub leaf_owner: Pubkey,
    #[serde(default, with = "value::pubkey::opt")]
    pub leaf_delegate: Option<Pubkey>,
    pub metadata: MetadataBubblegum,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let mint_ix = MintV1Builder::new()
        .leaf_delegate(input.leaf_delegate.unwrap_or(input.leaf_owner))
        .leaf_owner(input.leaf_owner)
        .merkle_tree(input.merkle_tree)
        .payer(input.payer.pubkey())
        .tree_config(input.tree_config)
        .tree_creator_or_delegate(input.creator_or_delegate.pubkey())
        .metadata(input.metadata.into())
        .instruction();

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer.clone_keypair(),
            input.creator_or_delegate.clone_keypair(),
        ]
        .into(),
        instructions: [mint_ix].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
