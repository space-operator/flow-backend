use crate::prelude::Pubkey;
use mpl_candy_machine_core::{CandyMachineData as MPLCandyMachineData, HiddenSettings};
use mpl_token_metadata::types::{Collection, DataV2, UseMethod, Uses};
use serde::{Deserialize, Serialize};

// pub mod approve_collection_authority;
// pub mod approve_use_authority;
pub mod arweave_file_upload;
pub mod arweave_nft_upload;
// pub mod candy_machine_v3;
pub mod create_master_edition;
pub mod create_metadata_account;
pub mod v1;
// pub mod get_left_uses;
// pub mod set_token_standard;
// pub mod sign_metadata;
// pub mod update_metadata_account;
// pub mod verify_collection;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NftDataV2 {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub seller_fee_basis_points: u16,
    pub creators: Option<Vec<NftCreator>>,
    pub collection: Option<NftCollection>,
    pub uses: Option<NftUses>,
}

impl From<NftDataV2> for DataV2 {
    fn from(v: NftDataV2) -> Self {
        Self {
            name: v.name,
            symbol: v.symbol,
            uri: v.uri,
            seller_fee_basis_points: v.seller_fee_basis_points,
            creators: v.creators.map(|v| v.into_iter().map(Into::into).collect()),
            collection: v.collection.map(Into::into),
            uses: v.uses.map(Into::into),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NftCollection {
    pub verified: bool,
    #[serde(with = "value::pubkey")]
    pub key: Pubkey,
}

impl From<NftCollection> for Collection {
    fn from(v: NftCollection) -> Self {
        Self {
            verified: v.verified,
            key: v.key,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NftMetadata {
    pub name: String,
    pub symbol: String,
    pub description: String,
    pub seller_fee_basis_points: u16,
    pub image: String,
    pub animation_url: Option<String>,
    pub external_url: Option<String>,
    pub attributes: Vec<NftMetadataAttribute>,
    pub properties: Option<NftMetadataProperties>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NftMetadataAttribute {
    pub trait_type: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NftMetadataProperties {
    pub files: Option<Vec<NftMetadataFile>>,
    pub category: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NftMetadataFile {
    pub uri: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct NftCreator {
    #[serde(with = "value::pubkey")]
    pub address: Pubkey,
    pub verified: Option<bool>,
    pub share: u8, // in percentage not basis points
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct NftUses {
    pub use_method: NftUseMethod,
    pub remaining: u64,
    pub total: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum NftUseMethod {
    Burn,
    Single,
    Multiple,
}

impl From<NftUses> for Uses {
    fn from(v: NftUses) -> Self {
        Uses {
            use_method: UseMethod::from(v.use_method.clone()),
            remaining: v.remaining,
            total: v.total,
        }
    }
}

impl From<NftUseMethod> for UseMethod {
    fn from(v: NftUseMethod) -> Self {
        match v {
            NftUseMethod::Burn => UseMethod::Burn,
            NftUseMethod::Single => UseMethod::Single,
            NftUseMethod::Multiple => UseMethod::Multiple,
        }
    }
}

impl From<NftCreator> for mpl_token_metadata::types::Creator {
    fn from(v: NftCreator) -> Self {
        mpl_token_metadata::types::Creator {
            address: v.address,
            verified: v.verified.is_some(),
            share: v.share,
        }
    }
}

impl From<NftCreator> for mpl_candy_machine_core::Creator {
    fn from(v: NftCreator) -> Self {
        mpl_candy_machine_core::Creator {
            address: v.address,
            verified: v.verified.is_some(),
            percentage_share: v.share,
        }
    }
}

// Candy machine configuration data.
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct CandyMachineDataAlias {
    /// Number of assets available
    pub items_available: u64,
    /// Symbol for the asset
    pub symbol: String,
    /// Secondary sales royalty basis points (0-10000)
    pub seller_fee_basis_points: u16,
    /// Max supply of each individual asset (default 0)
    pub max_supply: u64,
    /// Indicates if the asset is mutable or not (default yes)
    pub is_mutable: bool,
    /// List of creators
    pub creators: Vec<NftCreator>,
    /// Config line settings
    pub config_line_settings: Option<ConfigLineSettingsAlias>,
    /// Hidden setttings
    pub hidden_settings: Option<HiddenSettingsAlias>,
}

//
impl From<CandyMachineDataAlias> for MPLCandyMachineData {
    fn from(v: CandyMachineDataAlias) -> Self {
        MPLCandyMachineData {
            items_available: v.items_available,
            symbol: v.symbol,
            seller_fee_basis_points: v.seller_fee_basis_points,
            max_supply: v.max_supply,
            is_mutable: v.is_mutable,
            creators: v.creators.into_iter().map(|c| c.into()).collect(),
            config_line_settings: v.config_line_settings.map(|c| c.into()),
            hidden_settings: v.hidden_settings.map(|h| h.into()),
        }
    }
}

/// Hidden settings for large mints used with off-chain data.
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct HiddenSettingsAlias {
    /// Asset prefix name
    pub name: String,
    /// Shared URI
    pub uri: String,
    /// Hash of the hidden settings file
    pub hash: [u8; 32],
}

// implement From for HiddenSettingsAlias
impl From<HiddenSettingsAlias> for HiddenSettings {
    fn from(v: HiddenSettingsAlias) -> Self {
        HiddenSettings {
            name: v.name,
            uri: v.uri,
            hash: v.hash,
        }
    }
}

/// Config line settings to allocate space for individual name + URI.
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct ConfigLineSettingsAlias {
    /// Common name prefix
    pub prefix_name: String,
    /// Length of the remaining part of the name
    pub name_length: u32,
    /// Common URI prefix
    pub prefix_uri: String,
    /// Length of the remaining part of the URI
    pub uri_length: u32,
    /// Indicates whether to use a senquential index generator or not
    pub is_sequential: bool,
}

// implement From for ConfigLineSettingsAlias
impl From<ConfigLineSettingsAlias> for mpl_candy_machine_core::ConfigLineSettings {
    fn from(v: ConfigLineSettingsAlias) -> Self {
        mpl_candy_machine_core::ConfigLineSettings {
            prefix_name: v.prefix_name,
            name_length: v.name_length,
            prefix_uri: v.prefix_uri,
            uri_length: v.uri_length,
            is_sequential: v.is_sequential,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum CollectionDetails {
    V1 { size: u64 },
}

// implement From for CollectionDetails
impl From<CollectionDetails> for mpl_token_metadata::types::CollectionDetails {
    fn from(v: CollectionDetails) -> Self {
        match v {
            CollectionDetails::V1 { size } => {
                mpl_token_metadata::types::CollectionDetails::V1 { size }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, PartialOrd, Hash)]
pub enum TokenStandard {
    NonFungible,
    FungibleAsset,
    Fungible,
    NonFungibleEdition,
    ProgrammableNonFungible,
    ProgrammableNonFungibleEdition,
}

// Convert string to TokenStandard
impl From<String> for TokenStandard {
    fn from(v: String) -> Self {
        match v.as_str() {
            "non_fungible" => TokenStandard::NonFungible,
            "fungible_asset" => TokenStandard::FungibleAsset,
            "fungible" => TokenStandard::Fungible,
            "non_fungible_edition" => TokenStandard::NonFungibleEdition,
            "programmable_non_fungible" => TokenStandard::ProgrammableNonFungible,
            "programmable_non_fungible_edition" => TokenStandard::ProgrammableNonFungibleEdition,
            _ => panic!("Invalid token standard"),
        }
    }
}

// implement From for TokenStandard
impl From<TokenStandard> for mpl_token_metadata::types::TokenStandard {
    fn from(v: TokenStandard) -> Self {
        match v {
            TokenStandard::NonFungible => mpl_token_metadata::types::TokenStandard::NonFungible,
            TokenStandard::FungibleAsset => mpl_token_metadata::types::TokenStandard::FungibleAsset,
            TokenStandard::Fungible => mpl_token_metadata::types::TokenStandard::Fungible,
            TokenStandard::NonFungibleEdition => {
                mpl_token_metadata::types::TokenStandard::NonFungibleEdition
            }
            TokenStandard::ProgrammableNonFungible => {
                mpl_token_metadata::types::TokenStandard::ProgrammableNonFungible
            }
            TokenStandard::ProgrammableNonFungibleEdition => {
                mpl_token_metadata::types::TokenStandard::ProgrammableNonFungibleEdition
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum PrintSupply {
    Zero,
    Limited(u64),
    Unlimited,
}

// convert 0,u64, none to PrintSupply
impl From<Option<u64>> for PrintSupply {
    fn from(v: Option<u64>) -> Self {
        match v {
            Some(0) => PrintSupply::Zero,
            Some(supply) => PrintSupply::Limited(supply),
            None => PrintSupply::Unlimited,
        }
    }
}

// implement From for PrintSupply
impl From<PrintSupply> for mpl_token_metadata::types::PrintSupply {
    fn from(v: PrintSupply) -> Self {
        match v {
            PrintSupply::Zero => mpl_token_metadata::types::PrintSupply::Zero,
            PrintSupply::Limited(supply) => mpl_token_metadata::types::PrintSupply::Limited(supply),
            PrintSupply::Unlimited => mpl_token_metadata::types::PrintSupply::Unlimited,
        }
    }
}
