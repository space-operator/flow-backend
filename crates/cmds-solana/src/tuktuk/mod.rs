//! TukTuk Space Operator nodes
//!
//! Program ID: `tuktukUrfhXT6ZT77QTU8RQtvgL967uRuVagWF57zVA`
//! Repository: https://github.com/helium/tuktuk
//!
//! Uses tuktuk-program crate (anchor-lang / solana-program v2) internally,
//! with v2→v3 bridge functions for the workspace's solana v3 types.


/// System program ID (v3 Pubkey)
pub const SYSTEM_PROGRAM_ID: solana_pubkey::Pubkey =
    solana_pubkey::pubkey!("11111111111111111111111111111111");

pub mod pda;
pub mod types;

pub mod add_queue_authority_v0;
pub mod close_task_queue_v0;
pub mod dequeue_task_v0;
pub mod dummy_ix;
pub mod initialize_task_queue_v0;
pub mod initialize_tuktuk_config_v0;
pub mod queue_task_v0;
pub mod remove_queue_authority_v0;
pub mod return_tasks_v0;
pub mod run_task_v0;
pub mod update_task_queue_v0;

/// TukTuk program ID (v3 Pubkey)
pub const TUKTUK_PROGRAM_ID: solana_pubkey::Pubkey =
    solana_pubkey::pubkey!("tuktukUrfhXT6ZT77QTU8RQtvgL967uRuVagWF57zVA");

// Re-export shared v2↔v3 conversion helpers
pub use crate::solana_v2_compat::{to_pubkey_v2, to_instruction_v3};

/// Build a v3 Instruction from a list of v3 AccountMeta + raw instruction data (discriminator + borsh args).
pub fn build_ix(
    accounts: Vec<solana_instruction::AccountMeta>,
    data: Vec<u8>,
) -> solana_instruction::Instruction {
    solana_instruction::Instruction {
        program_id: TUKTUK_PROGRAM_ID,
        accounts,
        data,
    }
}

/// Convenience: create a writable signer account meta (v3)
#[inline]
pub fn account_meta_signer_mut(pk: &solana_pubkey::Pubkey) -> solana_instruction::AccountMeta {
    solana_instruction::AccountMeta {
        pubkey: *pk,
        is_signer: true,
        is_writable: true,
    }
}

/// Convenience: create a read-only signer account meta (v3)
#[inline]
pub fn account_meta_signer(pk: &solana_pubkey::Pubkey) -> solana_instruction::AccountMeta {
    solana_instruction::AccountMeta {
        pubkey: *pk,
        is_signer: true,
        is_writable: false,
    }
}

/// Convenience: create a writable non-signer account meta (v3)
#[inline]
pub fn account_meta_mut(pk: &solana_pubkey::Pubkey) -> solana_instruction::AccountMeta {
    solana_instruction::AccountMeta {
        pubkey: *pk,
        is_signer: false,
        is_writable: true,
    }
}

/// Convenience: create a read-only non-signer account meta (v3)
#[inline]
pub fn account_meta_readonly(pk: &solana_pubkey::Pubkey) -> solana_instruction::AccountMeta {
    solana_instruction::AccountMeta {
        pubkey: *pk,
        is_signer: false,
        is_writable: false,
    }
}

/// Compute the 8-byte Anchor instruction discriminator: sha256("global:<name>")[..8]
pub fn anchor_discriminator(name: &str) -> [u8; 8] {
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(format!("global:{name}").as_bytes());
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}
