use crate::prelude::*;
use mpl_bubblegum::instructions::MintToCollectionV1Builder;
use solana_sdk::pubkey::Pubkey;
use tracing::info;

use super::MetadataBubblegum;

// Command Name
const MINT_COMPRESSED_NFT: &str = "mint_cNFT_to_collection";

const DEFINITION: &str = flow_lib::node_definition!("compression/mint_to_collection_v1.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(MINT_COMPRESSED_NFT)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(MINT_COMPRESSED_NFT, |_| {
    build()
}));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::pubkey")]
    pub collection_mint: Pubkey,
    #[serde(with = "value::keypair")]
    pub collection_authority: Keypair,
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
    // Bubblegum address if none is provided
    let collection_authority_record_pda =
        mpl_token_metadata::accounts::CollectionAuthorityRecord::find_pda(
            &input.collection_mint,
            &input.collection_authority.pubkey(),
        )
        .0;

    let collection_metadata =
        mpl_token_metadata::accounts::Metadata::find_pda(&input.collection_mint).0;

    let collection_edition =
        mpl_token_metadata::accounts::MasterEdition::find_pda(&input.collection_mint).0;

    let mut metadata = input.metadata;
    metadata.collection = Some(super::Collection {
        verified: false,
        key: input.collection_mint.to_string(),
    });
    let mint_ix = MintToCollectionV1Builder::new()
        .tree_config(input.tree_config)
        .leaf_owner(input.leaf_owner)
        .leaf_delegate(input.leaf_delegate.unwrap_or(input.leaf_owner))
        .merkle_tree(input.merkle_tree)
        .payer(input.payer.pubkey())
        .tree_creator_or_delegate(input.creator_or_delegate.pubkey())
        .collection_authority(input.collection_authority.pubkey())
        .collection_authority_record_pda(Some(collection_authority_record_pda))
        .collection_mint(input.collection_mint)
        .collection_metadata(collection_metadata)
        .collection_edition(collection_edition)
        // Optional with defaults
        // .bubblegum_signer(bubblegum_signer)
        // .log_wrapper(log_wrapper)
        // .compression_program(compression_program)
        // .token_metadata_program(token_metadata_program)
        // .system_program(system_program)
        .metadata(metadata.into())
        .instruction();
    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer.clone_keypair(),
            input.creator_or_delegate.clone_keypair(),
            input.collection_authority.clone_keypair(),
        ]
        .into(),
        instructions: [mint_ix].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
