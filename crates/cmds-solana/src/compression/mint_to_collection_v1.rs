use super::MetadataBubblegum;
use crate::compression::get_leaf_schema_event;
use crate::prelude::*;

use bytes::Bytes;
use mpl_bubblegum::instructions::MintToCollectionV1Builder;
use solana_sdk::pubkey::Pubkey;
use tracing::info;

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

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub collection_mint: Pubkey,
    pub collection_authority: Wallet,
    pub creator_or_delegate: Wallet,
    #[serde(default = "value::default::bool_false")]
    pub is_delegate_authority: bool,
    #[serde_as(as = "AsPubkey")]
    pub tree_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub merkle_tree: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub leaf_owner: Pubkey,
    #[serde_as(as = "Option<AsPubkey>")]
    pub leaf_delegate: Option<Pubkey>,
    pub metadata: MetadataBubblegum,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde_as(as = "Option<AsSignature>")]
    signature: Option<Signature>,
    #[serde_as(as = "Option<AsPubkey>")]
    id: Option<Pubkey>,
    nonce: Option<u64>,
    creator_hash: Option<Bytes>,
    data_hash: Option<Bytes>,
    leaf_hash: Option<Bytes>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    // Bubblegum address if none is provided
    // TODO update to MetadataDelegateRecord::find_pda
    let collection_authority_record_pda = input.is_delegate_authority.then_some(
        mpl_token_metadata::accounts::CollectionAuthorityRecord::find_pda(
            &input.collection_mint,
            &input.collection_authority.pubkey(),
        )
        .0,
    );

    let collection_metadata =
        mpl_token_metadata::accounts::Metadata::find_pda(&input.collection_mint).0;

    let collection_edition =
        mpl_token_metadata::accounts::MasterEdition::find_pda(&input.collection_mint).0;

    let mut metadata = input.metadata;
    metadata.collection = Some(super::Collection {
        verified: false,
        key: input.collection_mint.to_string(),
    });
    info!("metadata: {:?}", metadata);
    info!(
        "collection authority {}",
        input.collection_authority.pubkey()
    );
    let mint_ix = MintToCollectionV1Builder::new()
        .tree_config(input.tree_config)
        .leaf_owner(input.leaf_owner)
        .leaf_delegate(input.leaf_delegate.unwrap_or(input.leaf_owner))
        .merkle_tree(input.merkle_tree)
        .payer(input.payer.pubkey())
        .tree_creator_or_delegate(input.creator_or_delegate.pubkey())
        .collection_authority(input.collection_authority.pubkey())
        .collection_authority_record_pda(collection_authority_record_pda)
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
            input.payer,
            input.creator_or_delegate,
            input.collection_authority,
        ]
        .into(),
        instructions: [mint_ix].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    let mut leaf_schema = None;
    if let Some(signature) = signature {
        leaf_schema = Some(get_leaf_schema_event(ctx, signature, true).await?.1);
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

#[cfg(test)]
mod tests {
    use anchor_lang::AnchorDeserialize;
    use mpl_bubblegum::LeafSchemaEvent;
    use spl_account_compression::{
        events::{ApplicationDataEvent, ApplicationDataEventV1},
        AccountCompressionEvent,
    };

    // use crate::compression::get_leaf_schema_event;

    /*
     * devnet resetted
    #[tokio::test]
    async fn test_get_leaf_schema() {
        tracing_subscriber::fmt::try_init().ok();
        const SIGNATURE: &str = "3a4asE3CbWjmpEBpxLwctqgF2BfwzUhsaDdrQS9ZnanNrWJYfxc8hWfow7gCF9MVjdB2SQ1svg8QujDMjNknufCU";
        let result = get_leaf_schema_event(<_>::default(), SIGNATURE.parse().unwrap(), true).await;
        dbg!(&result);
        result.unwrap();
    }
    */

    #[test]
    fn test_parse_instruction_data() {
        const DATA: &str = "2GJh7oUmkZKnjHLqLHwKU8DSRK2PJ6gqTyCkQzE4TvouB75xxWG7AbvGgMBvuw5QTbGAFKcUJGy9ftDfxdkk55MRYXruCpNqFcHp5GijZRzf3SCuHveuURcjqJ6owS9T9DBxxij7cQgfwfZzuR7LavH7MsiDatmpEj3NnmQdJRxDGm3S3JcsVqxy6Zd9zieqHDKR899HohKdxhJ7rKkZfbubHLxmH9vGvChktsHX5DywH1CxHnoiG6918Yjx1xPdLduc71Wx97C3xs7cw9pd9etUtYRCE";
        let bytes = bs58::decode(DATA).into_vec().unwrap();
        let event = AccountCompressionEvent::try_from_slice(&bytes).unwrap();
        let AccountCompressionEvent::ApplicationData(ApplicationDataEvent::V1(
            ApplicationDataEventV1 { application_data },
        )) = event
        else {
            panic!("wrong variant");
        };
        let leaf_schema = LeafSchemaEvent::try_from_slice(&application_data).unwrap();
        dbg!(leaf_schema);
    }
}
