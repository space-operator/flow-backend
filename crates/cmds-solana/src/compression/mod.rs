use crate::prelude::*;
use anchor_lang::AnchorDeserialize;
use anyhow::{anyhow, Context as _};
use flow_lib::command::CommandError;
use mpl_bubblegum::types::LeafSchema;
use mpl_bubblegum::types::{MetadataArgs, UpdateArgs};
use mpl_bubblegum::LeafSchemaEvent;
use serde::{Deserialize, Serialize};
use solana_client::rpc_config::RpcTransactionConfig;
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::UiParsedInstruction;
use solana_transaction_status::{
    option_serializer::OptionSerializer, UiInstruction, UiTransactionEncoding,
};
use spl_account_compression::{
    events::{ApplicationDataEvent, ApplicationDataEventV1},
    AccountCompressionEvent,
};
use std::str::FromStr;
use tracing::info;
pub mod burn;
pub mod create_tree;
pub mod mint_to_collection_v1;
pub mod mint_v1;
pub mod transfer;
pub mod types;
pub mod update;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum TokenProgramVersion {
    Original,
    Token2022,
}

impl From<TokenProgramVersion> for mpl_bubblegum::types::TokenProgramVersion {
    fn from(v: TokenProgramVersion) -> Self {
        match v {
            TokenProgramVersion::Original => mpl_bubblegum::types::TokenProgramVersion::Original,
            TokenProgramVersion::Token2022 => mpl_bubblegum::types::TokenProgramVersion::Token2022,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Creator {
    pub address: String,
    pub verified: bool,
    // In percentages, NOT basis points ;) Watch out!
    pub share: u8,
}

impl From<Creator> for mpl_bubblegum::types::Creator {
    fn from(v: Creator) -> Self {
        mpl_bubblegum::types::Creator {
            address: Pubkey::from_str(&v.address).unwrap(),
            verified: v.verified,
            share: v.share,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum TokenStandard {
    NonFungible,        // This is a master edition
    FungibleAsset,      // A token with metadata that can also have attrributes
    Fungible,           // A token with simple metadata
    NonFungibleEdition, // This is a limited edition
}

impl From<TokenStandard> for mpl_bubblegum::types::TokenStandard {
    fn from(v: TokenStandard) -> Self {
        match v {
            TokenStandard::NonFungible => mpl_bubblegum::types::TokenStandard::NonFungible,
            TokenStandard::FungibleAsset => mpl_bubblegum::types::TokenStandard::FungibleAsset,
            TokenStandard::Fungible => mpl_bubblegum::types::TokenStandard::Fungible,
            TokenStandard::NonFungibleEdition => {
                mpl_bubblegum::types::TokenStandard::NonFungibleEdition
            }
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum UseMethod {
    Burn,
    Multiple,
    Single,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Uses {
    // 17 bytes + Option byte
    pub use_method: UseMethod, //1
    pub remaining: u64,        //8
    pub total: u64,            //8
}

impl From<Uses> for mpl_bubblegum::types::Uses {
    fn from(v: Uses) -> Self {
        mpl_bubblegum::types::Uses {
            use_method: match v.use_method {
                UseMethod::Burn => mpl_bubblegum::types::UseMethod::Burn,
                UseMethod::Multiple => mpl_bubblegum::types::UseMethod::Multiple,
                UseMethod::Single => mpl_bubblegum::types::UseMethod::Single,
            },
            remaining: v.remaining,
            total: v.total,
        }
    }
}

#[repr(C)]
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Collection {
    pub verified: bool,
    pub key: String,
}

impl From<Collection> for mpl_bubblegum::types::Collection {
    fn from(v: Collection) -> Self {
        mpl_bubblegum::types::Collection {
            verified: v.verified,
            key: Pubkey::from_str(&v.key).unwrap(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct MetadataBubblegum {
    /// The name of the asset
    pub name: String,
    /// The symbol for the asset
    pub symbol: String,
    /// URI pointing to JSON representing the asset
    pub uri: String,
    /// Royalty basis points that goes to creators in secondary sales (0-10000)
    pub seller_fee_basis_points: u16,
    // Immutable, once flipped, all sales of this metadata are considered secondary.
    pub primary_sale_happened: bool,
    // Whether or not the data struct is mutable, default is not
    pub is_mutable: bool,
    /// nonce for easy calculation of editions, if present
    pub edition_nonce: Option<u8>,
    /// Since we cannot easily change Metadata, we add the new DataV2 fields here at the end.
    pub token_standard: Option<TokenStandard>,
    /// Collection
    pub collection: Option<Collection>,
    /// Uses
    pub uses: Option<Uses>,
    pub token_program_version: TokenProgramVersion,
    pub creators: Vec<Creator>,
}

// implement From MetadataBubblegum to MetadataArgs
impl From<MetadataBubblegum> for MetadataArgs {
    fn from(v: MetadataBubblegum) -> Self {
        Self {
            name: v.name,
            symbol: v.symbol,
            uri: v.uri,
            seller_fee_basis_points: v.seller_fee_basis_points,
            primary_sale_happened: v.primary_sale_happened,
            is_mutable: v.is_mutable,
            edition_nonce: v.edition_nonce,
            token_standard: v.token_standard.map(Into::into),
            collection: v.collection.map(Into::into),
            uses: v.uses.map(Into::into),
            token_program_version: v.token_program_version.into(),
            creators: v.creators.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<MetadataBubblegum> for UpdateArgs {
    fn from(v: MetadataBubblegum) -> Self {
        Self {
            name: Some(v.name),
            symbol: Some(v.symbol),
            uri: Some(v.uri),
            creators: Some(v.creators.into_iter().map(Into::into).collect()),
            seller_fee_basis_points: Some(v.seller_fee_basis_points),
            primary_sale_happened: Some(v.primary_sale_happened),
            is_mutable: Some(v.is_mutable),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GetAssetResponse<T> {
    pub id: String,
    pub result: T,
    pub jsonrpc: String,
}

pub async fn get_leaf_schema_event(
    ctx: Context,
    signature: Signature,
    is_mint_to_collection: bool,
) -> Result<(LeafSchemaEvent, LeafSchema), anyhow::Error> {
    let index = if is_mint_to_collection { 1 } else { 0 };

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
        .map(|meta| meta.inner_instructions);

    let tx_meta = match tx_meta {
        Some(OptionSerializer::Some(m)) => Some(m),
        Some(OptionSerializer::None) | Some(OptionSerializer::Skip) | None => None,
    };

    info!("tx_meta: {:?}", tx_meta);

    let inner_instruction = tx_meta
        .as_ref()
        .ok_or_else(|| CommandError::msg("tx_meta is None"))?
        .last() // Inserted 2 priority fee instructions at the beginning
        .ok_or_else(|| CommandError::msg("No inner instruction"))?
        .instructions
        .get(index)
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
    let bytes = bs58::decode(data_bs58).into_vec().context("bs58::decode")?;
    let event =
        AccountCompressionEvent::try_from_slice(&bytes).context("parse AccountCompressionEvent")?;
    let AccountCompressionEvent::ApplicationData(ApplicationDataEvent::V1(
        ApplicationDataEventV1 { application_data },
    )) = event
    else {
        return Err(anyhow!("wrong AccountCompressionEvent variant"));
    };
    let leaf_schema_event =
        LeafSchemaEvent::try_from_slice(&application_data).context("parse LeafSchemaEvent")?;

    let leaf_schema = leaf_schema_event.schema.clone();

    Ok((leaf_schema_event, leaf_schema))
}
