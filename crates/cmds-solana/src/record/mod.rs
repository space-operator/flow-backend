use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::Zeroable;
use flow_lib::SolanaNet;
use serde::Serialize;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{program_pack::IsInitialized, pubkey};
use {bytemuck::Pod, solana_sdk::program_error::ProgramError};

pub mod initialize_record_with_seed;
pub mod read_record;
pub mod write_to_record;


// TODO need to find correct mainnet 
pub const RECORD_MAINNET: Pubkey = pubkey!("recr1L3PCGKLbckBqMNcJhuuyU1zgo8nBhfLVsJNwr5");
// recr1L3PCGKLbckBqMNcJhuuyU1zgo8nBhfLVsJNwr5
pub const RECORD_DEVNET: Pubkey = pubkey!("6bCYkQ6pfLJMPivh17TV1Bqm3Q7GfkhU56iLtUiPXpK9");

pub const fn record_program_id(net: SolanaNet) -> Pubkey {
    match net {
        SolanaNet::Mainnet => crate::record::RECORD_MAINNET,
        // TODO testnet not deployed yet
        SolanaNet::Devnet => crate::record::RECORD_DEVNET,
        SolanaNet::Testnet => crate::record::RECORD_DEVNET,
    }
}

/// Instructions supported by the program
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum RecordInstruction {
    /// Create a new record
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writable]` Record account, must be uninitialized
    /// 1. `[]` Record authority
    Initialize,

    /// Write to the provided record account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writable]` Record account, must be previously initialized
    /// 1. `[signer]` Current record authority
    Write {
        /// Offset to start writing record, expressed as `u64`.
        offset: u64,
        /// Data to replace the existing record data
        data: Vec<u8>,
    },

    /// Update the authority of the provided record account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writable]` Record account, must be previously initialized
    /// 1. `[signer]` Current record authority
    /// 2. `[]` New record authority
    SetAuthority,

    /// Close the provided record account, draining lamports to recipient account
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writable]` Record account, must be previously initialized
    /// 1. `[signer]` Record authority
    /// 2. `[]` Receiver of account lamports
    CloseAccount,
}

/// Struct wrapping data and providing metadata
/// #[repr(C)]

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize)]
pub struct RecordData {
    /// Struct version, allows for upgrades to the program
    pub version: u8,

    /// The account allowed to update the data
    pub authority: Pubkey,
}

impl RecordData {
    /// Version to fill in on new created accounts
    pub const CURRENT_VERSION: u8 = 1;

    /// Start of writable account data, after version and authority
    pub const WRITABLE_START_INDEX: usize = 33;
}

impl IsInitialized for RecordData {
    /// Is initialized
    fn is_initialized(&self) -> bool {
        self.version == Self::CURRENT_VERSION
    }
}

/// Convert a slice of bytes into a `Pod` (zero copy)
pub fn pod_from_bytes<T: Pod>(bytes: &[u8]) -> Result<&T, ProgramError> {
    bytemuck::try_from_bytes(bytes).map_err(|_| ProgramError::InvalidArgument)
}
