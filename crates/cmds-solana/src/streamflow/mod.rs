use borsh::{BorshDeserialize, BorshSerialize};
use flow_lib::SolanaNet;
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use solana_sdk::pubkey;

pub mod create;

pub const STREAMFLOW_PROGRAM_ID: Pubkey = pubkey!("strmRqUCoQUgGUan5YhzUZa6KqdzwX5L6FpUxfmKg5m");
pub const STREAMFLOW_DEVNET_PROGRAM_ID: Pubkey =
    pubkey!("HqDGZjaVRXJ9MGRQEw7qDc2rAr6iH1n1kAQdCZaCMfMZ");

pub const fn streamflow_program_id(net: SolanaNet) -> Pubkey {
    match net {
        SolanaNet::Mainnet => crate::streamflow::STREAMFLOW_PROGRAM_ID,
        // TODO testnet not deployed yet
        SolanaNet::Testnet => crate::streamflow::STREAMFLOW_DEVNET_PROGRAM_ID,
        SolanaNet::Devnet => crate::streamflow::STREAMFLOW_DEVNET_PROGRAM_ID,
    }
}

/// Streamflow Treasury address, by default receives 0.25% of tokens deposited
pub const STRM_TREASURY: &str = "5SEpbdjFK5FxwTvfsGMXVQTD2v4M2c5tyRTxhdsPkgDw";
/// Streamflow Withdrawor address, this account will process withdrawals
pub const WITHDRAWOR_ADDRESS: &str = "wdrwhnCv4pzW8beKsbPa4S2UDZrXenjg16KJdKSpb5u";
/// Address of Fee Oracle that stores information about fees for speficic partners
pub const FEE_ORACLE_ADDRESS: &str = "B743wFVk2pCYhV91cn287e1xY7f1vt4gdY48hhNiuQmT";

/// Prefix used to derive Escrow account address
pub const ESCROW_SEED_PREFIX: &[u8] = b"strm";

/// Size of Stream metadata
pub const METADATA_LEN: usize = 1104;

pub fn find_escrow_account(seed: &[u8], pid: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ESCROW_SEED_PREFIX, seed], pid)
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateData {
    start_time: u64,
    net_amount_deposited: u64,
    period: u64,
    amount_per_period: u64,
    cliff: u64,
    cliff_amount: u64,
    cancelable_by_sender: bool,
    cancelable_by_recipient: bool,
    automatic_withdrawal: bool,
    transferable_by_sender: bool,
    transferable_by_recipient: bool,
    can_topup: bool,
    stream_name: [u8; 64],
    withdraw_frequency: u64,
    pausable: Option<bool>,
    can_update_rate: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateDataInput {
    start_time: u64,
    net_amount_deposited: u64,
    period: u64,
    amount_per_period: u64,
    cliff: u64,
    cliff_amount: u64,
    cancelable_by_sender: bool,
    cancelable_by_recipient: bool,
    automatic_withdrawal: bool,
    transferable_by_sender: bool,
    transferable_by_recipient: bool,
    can_topup: bool,
    stream_name: String,
    withdraw_frequency: u64,
    pausable: Option<bool>,
    can_update_rate: Option<bool>,
}

impl From<CreateDataInput> for CreateData {
    fn from(input: CreateDataInput) -> Self {
        let mut array = [0; 64];
        let bytes = input.stream_name.as_bytes();

        array[..bytes.len()].copy_from_slice(bytes);

        CreateData {
            start_time: input.start_time,
            net_amount_deposited: input.net_amount_deposited,
            period: input.period,
            amount_per_period: input.amount_per_period,
            cliff: input.cliff,
            cliff_amount: input.cliff_amount,
            cancelable_by_sender: input.cancelable_by_sender,
            cancelable_by_recipient: input.cancelable_by_recipient,
            automatic_withdrawal: input.automatic_withdrawal,
            transferable_by_sender: input.transferable_by_sender,
            transferable_by_recipient: input.transferable_by_recipient,
            can_topup: input.can_topup,
            stream_name: array,
            withdraw_frequency: input.withdraw_frequency,
            pausable: input.pausable,
            can_update_rate: input.can_update_rate,
        }
    }
}
