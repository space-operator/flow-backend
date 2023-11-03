use borsh::{BorshDeserialize, BorshSerialize};
use primitive_types::U256;

use super::{token_bridge::Address, ForeignAddress};

pub mod complete_native;
pub mod complete_wrapped;
pub mod complete_wrapped_meta;
pub mod eth;
pub mod transfer_native;
pub mod transfer_wrapped;

#[repr(u8)]
#[derive(BorshSerialize, BorshDeserialize)]
enum NFTBridgeInstructions {
    Initialize,
    CompleteNative,
    CompleteWrapped,
    CompleteWrappedMeta,
    TransferWrapped,
    TransferNative,
    RegisterChain,
    UpgradeContract,
}

pub type ChainID = u16;

#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct TransferWrappedData {
    pub nonce: u32,
    pub target_address: Address,
    pub target_chain: ChainID,
}

#[derive(PartialEq, Debug, Clone)]
pub struct PayloadTransfer {
    // Address of the token. Left-zero-padded if shorter than 32 bytes
    pub token_address: ForeignAddress,
    // Chain ID of the token
    pub token_chain: ChainID,
    // Symbol of the token
    pub symbol: String,
    // Name of the token
    pub name: String,
    // TokenID of the token (big-endian uint256)
    pub token_id: U256,
    // URI of the token metadata
    pub uri: String,
    // Address of the recipient. Left-zero-padded if shorter than 32 bytes
    pub to: Address,
    // Chain ID of the recipient
    pub to_chain: ChainID,
}

#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct CompleteWrappedData {}

#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct CompleteWrappedMetaData {}

#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct TransferNativeData {
    pub nonce: u32,
    pub target_address: Address,
    pub target_chain: ChainID,
}

#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct CompleteNativeData {}
