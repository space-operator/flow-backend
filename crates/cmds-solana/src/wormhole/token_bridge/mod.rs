use crate::wormhole::ForeignAddress;
use borsh::{BorshDeserialize, BorshSerialize};

use flow_lib::Context;
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use wormhole_sdk::Amount;

pub mod attest;
pub mod complete_native;
pub mod complete_transfer_wrapped;
pub mod create_wrapped;
pub mod initialize;
pub mod transfer_native;
pub mod transfer_wrapped;

pub mod eth;

#[repr(u8)]
#[derive(BorshSerialize, BorshDeserialize)]
enum TokenBridgeInstructions {
    Initialize,
    AttestToken,
    CompleteNative,
    CompleteWrapped,
    TransferWrapped,
    TransferNative,
    RegisterChain,
    CreateWrapped,
    UpgradeContract,
    CompleteNativeWithPayload,
    CompleteWrappedWithPayload,
    TransferWrappedWithPayload,
    TransferNativeWithPayload,
}

#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct AttestTokenData {
    pub nonce: u32,
}

#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct CreateWrappedData {}

#[derive(PartialEq, Debug)]
pub struct PayloadAssetMeta {
    /// Address of the token. Left-zero-padded if shorter than 32 bytes
    pub token_address: ForeignAddress,
    /// Chain ID of the token
    pub token_chain: ChainID,
    /// Number of decimals of the token
    pub decimals: u8,
    /// Symbol of the token
    pub symbol: String,
    /// Name of the token
    pub name: String,
}

#[derive(
    Serialize, Deserialize, BorshDeserialize, BorshSerialize, Default, PartialEq, Debug, Clone,
)]
pub struct Address(pub [u8; 32]);

// implement from wormhole_sdk::Address to Address
impl From<wormhole_sdk::Address> for Address {
    fn from(address: wormhole_sdk::Address) -> Self {
        let mut addr = [0u8; 32];
        addr.copy_from_slice(&address.0);
        Address(addr)
    }
}

pub type ChainID = u16;

#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct CompleteWrappedData {}

#[derive(PartialEq, Debug, Clone)]
pub struct PayloadTransfer {
    /// Amount being transferred (big-endian uint256)
    pub amount: Amount,
    /// Address of the token. Left-zero-padded if shorter than 32 bytes
    pub token_address: ForeignAddress,
    /// Chain ID of the token
    pub token_chain: ChainID,
    /// Address of the recipient. Left-zero-padded if shorter than 32 bytes
    pub to: Address,
    /// Chain ID of the recipient
    pub to_chain: ChainID,
    /// Amount of tokens (big-endian uint256) that the user is willing to pay as relayer fee. Must be <= Amount.
    pub fee: Amount,
}

#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct CompleteWrappedWithPayloadData {}

#[derive(PartialEq, Debug, Clone)]
pub struct PayloadTransferWithPayload {
    /// Amount being transferred (big-endian uint256)
    pub amount: Amount,
    /// Address of the token. Left-zero-padded if shorter than 32 bytes
    pub token_address: ForeignAddress,
    /// Chain ID of the token
    pub token_chain: ChainID,
    /// Address of the recipient. Left-zero-padded if shorter than 32 bytes
    pub to: Address,
    /// Chain ID of the recipient
    pub to_chain: ChainID,
    /// Sender of the transaction
    pub from_address: Address,
    /// Arbitrary payload
    pub payload: Vec<u8>,
}

#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct TransferWrappedData {
    pub nonce: u32,
    pub amount: u64,
    pub fee: u64,
    pub target_address: Address,
    pub target_chain: ChainID,
}

#[derive(Default, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct SequenceTracker {
    pub sequence: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct TransferNativeData {
    pub nonce: u32,
    pub amount: u64,
    pub fee: u64,
    pub target_address: Address,
    pub target_chain: ChainID,
}

#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct CompleteNativeData {}

pub async fn get_sequence_number(ctx: &Context, sequence: Pubkey) -> SequenceTracker {
    let sequence_account: solana_sdk::account::Account =
        ctx.solana_client.get_account(&sequence).await.unwrap();
    let sequence_data: SequenceTracker =
        SequenceTracker::try_from_slice(&sequence_account.data).unwrap();
    sequence_data
}
