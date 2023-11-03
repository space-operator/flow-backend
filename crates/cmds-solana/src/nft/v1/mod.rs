use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

pub mod create_v1;
pub mod delegate_v1;
pub mod update_v1;
pub mod verify_collection_v1;
// pub mod transfer_v1;
pub mod mint_v1;
pub mod verify_creator_v1;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct AuthorizationData {
    pub payload: Payload,
}

impl From<AuthorizationData> for mpl_token_metadata::types::AuthorizationData {
    fn from(authorization_data: AuthorizationData) -> Self {
        Self {
            payload: authorization_data.payload.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Payload {
    pub map: HashMap<String, PayloadType>,
}

impl From<Payload> for mpl_token_metadata::types::Payload {
    fn from(payload: Payload) -> Self {
        let mut map = std::collections::HashMap::new();
        for (key, value) in payload.map {
            map.insert(key, value.into());
        }
        Self { map }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum PayloadType {
    Pubkey(Pubkey),
    Seeds(SeedsVec),
    MerkleProof(LeafInfo),
    Number(u64),
}

impl From<PayloadType> for mpl_token_metadata::types::PayloadType {
    fn from(payload_type: PayloadType) -> Self {
        match payload_type {
            PayloadType::Pubkey(pubkey) => Self::Pubkey(pubkey),
            PayloadType::Seeds(seeds_vec) => Self::Seeds(seeds_vec.into()),
            PayloadType::MerkleProof(leaf_info) => Self::MerkleProof(leaf_info.into()),
            PayloadType::Number(number) => Self::Number(number),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct SeedsVec {
    pub seeds: Vec<Vec<u8>>,
}

impl From<SeedsVec> for mpl_token_metadata::types::SeedsVec {
    fn from(seeds_vec: SeedsVec) -> Self {
        Self {
            seeds: seeds_vec.seeds,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct LeafInfo {
    pub leaf: [u8; 32],
    pub proof: Vec<[u8; 32]>,
}

impl From<LeafInfo> for mpl_token_metadata::types::LeafInfo {
    fn from(leaf_info: LeafInfo) -> Self {
        Self {
            leaf: leaf_info.leaf,
            proof: leaf_info.proof,
        }
    }
}
