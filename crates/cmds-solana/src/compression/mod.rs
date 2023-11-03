pub mod create_tree;
pub mod mint_v1;
use std::str::FromStr;

use mpl_bubblegum::state::metaplex_adapter::MetadataArgs;
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum TokenProgramVersion {
    Original,
    Token2022,
}

impl From<TokenProgramVersion> for mpl_bubblegum::state::metaplex_adapter::TokenProgramVersion {
    fn from(v: TokenProgramVersion) -> Self {
        match v {
            TokenProgramVersion::Original => {
                mpl_bubblegum::state::metaplex_adapter::TokenProgramVersion::Original
            }
            TokenProgramVersion::Token2022 => {
                mpl_bubblegum::state::metaplex_adapter::TokenProgramVersion::Token2022
            }
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

impl From<Creator> for mpl_bubblegum::state::metaplex_adapter::Creator {
    fn from(v: Creator) -> Self {
        mpl_bubblegum::state::metaplex_adapter::Creator {
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

impl From<TokenStandard> for mpl_bubblegum::state::metaplex_adapter::TokenStandard {
    fn from(v: TokenStandard) -> Self {
        match v {
            TokenStandard::NonFungible => {
                mpl_bubblegum::state::metaplex_adapter::TokenStandard::NonFungible
            }
            TokenStandard::FungibleAsset => {
                mpl_bubblegum::state::metaplex_adapter::TokenStandard::FungibleAsset
            }
            TokenStandard::Fungible => {
                mpl_bubblegum::state::metaplex_adapter::TokenStandard::Fungible
            }
            TokenStandard::NonFungibleEdition => {
                mpl_bubblegum::state::metaplex_adapter::TokenStandard::NonFungibleEdition
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

impl From<Uses> for mpl_bubblegum::state::metaplex_adapter::Uses {
    fn from(v: Uses) -> Self {
        mpl_bubblegum::state::metaplex_adapter::Uses {
            use_method: match v.use_method {
                UseMethod::Burn => mpl_bubblegum::state::metaplex_adapter::UseMethod::Burn,
                UseMethod::Multiple => mpl_bubblegum::state::metaplex_adapter::UseMethod::Multiple,
                UseMethod::Single => mpl_bubblegum::state::metaplex_adapter::UseMethod::Single,
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

impl From<Collection> for mpl_bubblegum::state::metaplex_adapter::Collection {
    fn from(v: Collection) -> Self {
        mpl_bubblegum::state::metaplex_adapter::Collection {
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
