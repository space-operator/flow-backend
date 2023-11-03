use std::str::FromStr;

use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

pub mod create_install;
pub mod create_permissioned_install;
pub mod create_xnft;
pub mod delete_install;
pub mod grant_access;
pub mod revoke_access;

// #[derive(Deserialize, Serialize, Debug)]
// pub enum Kind {
//     App,
//     Collectible,
// }

// #[derive(Deserialize, Serialize, Debug)]
// pub enum L1 {
//     Solana,
//     Ethereum,
// }

#[derive(Deserialize, Serialize, Debug)]
pub enum Tag {
    None,
    Defi,
    Game,
    Nft,
}

impl From<Tag> for xnft::state::Tag {
    fn from(tag: Tag) -> Self {
        match tag {
            Tag::None => Self::None,
            Tag::Defi => Self::Defi,
            Tag::Game => Self::Game,
            Tag::Nft => Self::Nfts,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CuratorStatus {
    /// The pubkey of the `Curator` program account (32).
    pub pubkey: Pubkey,
    /// Whether the curator's authority has verified the assignment (1).
    pub verified: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CreatorsParam {
    pub address: String,
    pub share: u8,
}

impl From<CreatorsParam> for xnft::state::CreatorsParam {
    fn from(param: CreatorsParam) -> Self {
        Self {
            address: Pubkey::from_str(&param.address).unwrap(),
            share: param.share,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CreateXnftParams {
    pub creators: Vec<CreatorsParam>,
    pub curator: Option<String>, // Some("...") values are only relevant for Kind::App xNFTs
    pub install_authority: Option<String>, // Some("...") values are only relevant for Kind::App xNFTs
    pub install_price: u64,
    pub install_vault: String,
    pub seller_fee_basis_points: u16,
    pub supply: Option<u64>, // Some("...") values are only relevant for Kind::App xNFTs
    pub symbol: String,
    pub tag: Tag,
    pub uri: String,
}

impl From<CreateXnftParams> for xnft::state::CreateXnftParams {
    fn from(params: CreateXnftParams) -> Self {
        Self {
            creators: params
                .creators
                .into_iter()
                .map(|param| param.into())
                .collect(),
            curator: params
                .curator
                .map(|curator| Pubkey::from_str(&curator).unwrap()),
            install_authority: params
                .install_authority
                .map(|install_authority| Pubkey::from_str(&install_authority).unwrap()),
            install_price: params.install_price,
            install_vault: Pubkey::from_str(&params.install_vault).unwrap(),
            seller_fee_basis_points: params.seller_fee_basis_points,
            supply: params.supply,
            symbol: params.symbol,
            tag: params.tag.into(),
            uri: params.uri,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct UpdateParams {
    pub install_authority: Option<Pubkey>, // Some("...") values are only relevant for Kind::App xNFTs
    // Will remove any existing install authority is given `None`
    pub install_price: Option<u64>, // Some("...") values are only relevant for Kind::App xNFTs
    pub install_vault: Option<Pubkey>, // Some("...") values are only relevant for Kind::App xNFTs
    pub name: Option<String>,       // Some("...") values are only relevant for Kind::App xNFTs
    pub supply: Option<u64>,        // Some("...") values are only relevant for Kind::App xNFTs
    pub tag: Option<Tag>,
    pub uri: Option<String>,
}

impl From<UpdateParams> for xnft::state::UpdateParams {
    fn from(params: UpdateParams) -> Self {
        Self {
            install_authority: params.install_authority,
            install_price: params.install_price,
            install_vault: params.install_vault,
            name: params.name,
            supply: params.supply,
            tag: params.tag.map(|tag| tag.into()),
            uri: params.uri,
        }
    }
}
