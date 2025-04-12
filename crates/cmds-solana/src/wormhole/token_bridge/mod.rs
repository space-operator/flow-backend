use std::fmt;

use crate::wormhole::ForeignAddress;
use borsh::{BorshDeserialize, BorshSerialize};

use byteorder::{ByteOrder, LittleEndian};
use flow_lib::{Context, context::CommandContextX};
use serde::{Deserialize, Serialize};
use solana_commitment_config::CommitmentConfig;
use solana_program::pubkey::Pubkey;
use tracing::info;
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

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }

        Ok(())
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

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone)]
pub struct TransferTokensArgs {
    pub nonce: u32,
    pub amount: u64,
    pub relayer_fee: u64,
    pub recipient: [u8; 32],
    pub recipient_chain: u16,
}

#[derive(Default, BorshSerialize, BorshDeserialize, Serialize)]
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

pub async fn get_sequence_number(
    ctx: &CommandContextX,
    sequence: Pubkey,
) -> Result<SequenceTracker, crate::Error> {
    let commitment = CommitmentConfig::confirmed();

    let response = ctx
        .solana_client()
        .get_account_with_commitment(&sequence, commitment)
        .await
        .map_err(|e| {
            tracing::error!("Error: {:?}", e);
            crate::Error::AccountNotFound(sequence)
        })?;

    info!("response: {:?}", response);
    let sequence_account = match response.value {
        Some(account) => account,
        None => return Err(crate::Error::AccountNotFound(sequence)),
    };

    let mut sequence_data: &[u8] = &sequence_account.data;
    let sequence_data: SequenceTracker =
        SequenceTracker::deserialize(&mut sequence_data).map_err(|_| {
            tracing::error!(
                "Invalid data for sequence: {:?}",
                crate::Error::InvalidAccountData(sequence)
            );
            crate::Error::InvalidAccountData(sequence)
        })?;
    info!("sequence_data: {:?}", sequence_data.sequence);
    Ok(sequence_data)
}

// https://github.com/wormhole-foundation/connect-sdk/blob/dc90598ecadea0319a83a983ae87667f44a3b5f2/platforms/solana/protocols/core/src/core.ts#L294C17-L294C17
pub async fn get_sequence_number_from_message(
    ctx: &CommandContextX,
    message: Pubkey,
) -> Result<String, crate::Error> {
    let commitment = CommitmentConfig::confirmed();

    let response = ctx
        .solana_client()
        .get_account_with_commitment(&message, commitment)
        .await
        .map_err(|e| {
            tracing::error!("Error: {:?}", e);
            crate::Error::AccountNotFound(message)
        })?;

    info!("response: {:?}", response);
    let sequence_account = match response.value {
        Some(account) => account,
        None => return Err(crate::Error::AccountNotFound(message)),
    };

    let sequence_data: &[u8] = &sequence_account.data;
    let sequence: u64 = LittleEndian::read_u64(&sequence_data[49..57]);

    info!("sequence_data: {:?}", sequence);
    Ok(sequence.to_string())
}
