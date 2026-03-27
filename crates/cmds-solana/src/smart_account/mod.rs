//! Squads Smart Account Program (Multisig v4) nodes
//!
//! Program ID: `SMRTzfY6DfH5ik3TKiyLFfXexV8uSG3d2UksSCYdunG`
//! Repository: https://github.com/Squads-Protocol/smart-account-program
//!
//! Direct instruction construction (no SDK crate dependency).
//! This is an Anchor-based program using 8-byte SHA-256 discriminators.

use crate::prelude::*;
use solana_program::pubkey;

pub mod activate_proposal;
pub mod add_signer;
pub mod add_spending_limit;
pub mod add_transaction_to_batch;
pub mod approve_proposal;
pub mod cancel_proposal;
pub mod change_threshold;
pub mod close_batch;
pub mod close_batch_transaction;
pub mod close_settings_transaction;
pub mod close_transaction;
pub mod close_transaction_buffer;
pub mod create_batch;
pub mod create_proposal;
pub mod create_settings_transaction;
pub mod create_smart_account;
pub mod create_transaction;
pub mod create_transaction_buffer;
pub mod create_transaction_from_buffer;
pub mod execute_batch_transaction;
pub mod execute_settings_transaction;
pub mod execute_transaction;
pub mod extend_transaction_buffer;
pub mod pda;
pub mod reject_proposal;
pub mod remove_signer;
pub mod remove_spending_limit;
pub mod set_archival_authority;
pub mod set_settings_authority;
pub mod set_time_lock;
pub mod use_spending_limit;

/// Squads Smart Account Program ID
pub const PROGRAM_ID: Pubkey = pubkey!("SMRTzfY6DfH5ik3TKiyLFfXexV8uSG3d2UksSCYdunG");

/// Compute the Anchor 8-byte discriminator for an instruction.
///
/// `sha256("global:<snake_case_name>")[..8]`
pub fn anchor_sighash(name: &str) -> [u8; 8] {
    let preimage = format!("global:{name}");
    let hash = solana_program::hash::hash(preimage.as_bytes());
    hash.to_bytes()[..8].try_into().unwrap()
}

/// Build a Smart Account instruction: 8-byte Anchor discriminator + args data.
pub fn build_instruction(
    ix_name: &str,
    accounts: Vec<solana_program::instruction::AccountMeta>,
    args_data: Vec<u8>,
) -> Instruction {
    let disc = anchor_sighash(ix_name);
    let mut data = Vec::with_capacity(8 + args_data.len());
    data.extend_from_slice(&disc);
    data.extend_from_slice(&args_data);
    Instruction {
        program_id: PROGRAM_ID,
        accounts,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchor_sighash_deterministic() {
        let d1 = anchor_sighash("create_smart_account");
        let d2 = anchor_sighash("create_smart_account");
        assert_eq!(d1, d2);
    }

    #[test]
    fn test_anchor_sighash_different() {
        let d1 = anchor_sighash("create_smart_account");
        let d2 = anchor_sighash("create_proposal");
        assert_ne!(d1, d2);
    }

    #[test]
    fn test_build_instruction_layout() {
        let ix = build_instruction("create_smart_account", vec![], vec![0xAA, 0xBB]);
        assert_eq!(ix.program_id, PROGRAM_ID);
        assert_eq!(ix.data.len(), 10); // 8-byte disc + 2-byte args
    }
}
