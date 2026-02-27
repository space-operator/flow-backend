use crate::{mpl_bubblegum::get_leaf_schema_event, prelude::*};
use bytes::Bytes;
use mpl_bubblegum::instructions::MintV1Builder;
use solana_program::pubkey::Pubkey;
use tracing::info;

use super::MetadataBubblegum;

// Command Name
const NAME: &str = "mint_compressed_NFT";

const DEFINITION: &str = flow_lib::node_definition!("mpl_bubblegum/mint_compressed_NFT.jsonc");

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
    pub payer: Wallet,
    pub creator_or_delegate: Wallet,
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
    #[serde(with = "value::pubkey::opt")]
    id: Option<Pubkey>,
    nonce: Option<u64>,
    creator_hash: Option<Bytes>,
    data_hash: Option<Bytes>,
    leaf_hash: Option<Bytes>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
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
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.creator_or_delegate].into(),
        instructions: [mint_ix].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    let mut leaf_schema = None;
    if let Some(signature) = signature {
        leaf_schema = Some(get_leaf_schema_event(ctx, signature, false).await?.1);
        info!("{:?}", leaf_schema);
    }

    let id = leaf_schema.as_ref().map(|schema| schema.id());
    let nonce = leaf_schema.as_ref().map(|schema| schema.nonce());
    let data_hash = leaf_schema
        .as_ref()
        .map(|schema| bytes::Bytes::copy_from_slice(&schema.data_hash()));
    let leaf_hash = leaf_schema
        .as_ref()
        .map(|schema| bytes::Bytes::copy_from_slice(&schema.hash()));
    let creator_hash =
        leaf_schema.map(|schema| bytes::Bytes::copy_from_slice(&schema.creator_hash()));

    Ok(Output {
        signature,
        id,
        nonce,
        creator_hash,
        data_hash,
        leaf_hash,
    })
}
