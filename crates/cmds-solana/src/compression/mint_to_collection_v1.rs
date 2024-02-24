use super::MetadataBubblegum;
use crate::prelude::*;
use anchor_lang::AnchorDeserialize;
use anyhow::{anyhow, Context as _};
use mpl_bubblegum::instructions::MintToCollectionV1Builder;
use mpl_bubblegum::LeafSchemaEvent;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use solana_transaction_status::UiParsedInstruction;
use solana_transaction_status::{
    option_serializer::OptionSerializer, UiInstruction, UiTransactionEncoding,
};
use spl_account_compression::{
    events::{ApplicationDataEvent, ApplicationDataEventV1},
    AccountCompressionEvent,
};
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
    #[serde(default = "value::default::bool_false")]
    pub is_delegate_authority: bool,
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

async fn get_leaf_schema(
    ctx: Context,
    signature: Signature,
) -> Result<LeafSchemaEvent, anyhow::Error> {
    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(CommitmentConfig::confirmed()),
        // we only send "legacy" tx at the moment
        max_supported_transaction_version: None,
    };
    let tx_meta = ctx
        .solana_client
        .get_transaction_with_config(&signature, config)
        .await?
        .transaction
        .meta
        .and_then(|meta| Some(meta.inner_instructions));

    let tx_meta = match tx_meta.unwrap() {
        OptionSerializer::None => None,
        OptionSerializer::Some(m) => Some(m),
        OptionSerializer::Skip => None,
    };

    info!("tx_meta: {:?}", tx_meta);

    let inner_instruction = tx_meta
        .as_ref()
        .ok_or_else(|| CommandError::msg("tx_meta is None"))?
        .last() // Inserted 2 priority fee instructions at the beginning
        .ok_or_else(|| CommandError::msg("No inner instruction"))?
        .instructions
        .get(1)
        .ok_or_else(|| CommandError::msg("No instruction at index 1"))?
        .clone();

    let data_bs58 = match inner_instruction {
        UiInstruction::Parsed(UiParsedInstruction::PartiallyDecoded(i)) => i.data,
        _ => {
            return Err(anyhow!(
                "expected UiInstruction::Parsed(PartiallyDecoded(_)), got {:?}",
                inner_instruction
            ));
        }
    };
    let bytes = bs58::decode(data_bs58).into_vec()?;
    let event =
        AccountCompressionEvent::try_from_slice(&bytes).context("parse AccountCompressionEvent")?;
    let AccountCompressionEvent::ApplicationData(ApplicationDataEvent::V1(
        ApplicationDataEventV1 { application_data },
    )) = event
    else {
        return Err(anyhow!("wrong AccountCompressionEvent variant"));
    };
    let leaf_schema =
        LeafSchemaEvent::try_from_slice(&application_data).context("parse LeafSchemaEvent")?;

    Ok(leaf_schema)
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
            input.payer.clone_keypair(),
            input.creator_or_delegate.clone_keypair(),
            input.collection_authority.clone_keypair(),
        ]
        .into(),
        instructions: [mint_ix].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    if let Some(signature) = signature {
        let leaf_schema = get_leaf_schema(ctx, signature).await;
        info!("{:?}", leaf_schema);
    }

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_leaf_schema() {
        tracing_subscriber::fmt::try_init().ok();
        const SIGNATURE: &str = "3a4asE3CbWjmpEBpxLwctqgF2BfwzUhsaDdrQS9ZnanNrWJYfxc8hWfow7gCF9MVjdB2SQ1svg8QujDMjNknufCU";
        let result = get_leaf_schema(<_>::default(), SIGNATURE.parse().unwrap()).await;
        dbg!(&result);
        result.unwrap();
    }

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
