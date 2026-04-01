//! MagicBlock Ephemeral SPL Token — Space Operator nodes.
//!
//! Program ID: `SPLxh1LVZzEkX99H6rqYizhytLWPZVV296zyYDPagv2`
//! Repository: https://github.com/magicblock-labs/ephemeral-spl-token

use solana_program::pubkey::Pubkey;

/// Ephemeral SPL Token Program ID
pub const ETOKEN_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("SPLxh1LVZzEkX99H6rqYizhytLWPZVV296zyYDPagv2");

/// MagicBlock Delegation Program ID
pub const DELEGATION_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSS");

/// MagicBlock Permission Program ID (ACL)
pub const PERMISSION_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("ACLseoPoyC3cBqoUtkbjZ4aDrkurZW86v19pXz2XQnp1");

/// MagicBlock Magic Program ID (commit/undelegate CPI target)
pub const MAGIC_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("Magic11111111111111111111111111111111111111");

/// MagicBlock Magic Context account (injected by commit macro)
pub const MAGIC_CONTEXT_ID: Pubkey =
    Pubkey::from_str_const("MagicContext1111111111111111111111111111111");

// ── PDA Derivation ──────────────────────────────────────────────────────────

pub mod pda {
    use super::{ETOKEN_PROGRAM_ID, Pubkey};

    /// Ephemeral ATA PDA: seeds = [user, mint]
    pub fn ephemeral_ata(user: &Pubkey, mint: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[user.as_ref(), mint.as_ref()], &ETOKEN_PROGRAM_ID).0
    }

    /// Global Vault PDA: seeds = [mint]
    pub fn global_vault(mint: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[mint.as_ref()], &ETOKEN_PROGRAM_ID).0
    }

    /// Vault Ephemeral ATA PDA: seeds = [vault, mint]
    pub fn vault_ephemeral_ata(vault: &Pubkey, mint: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[vault.as_ref(), mint.as_ref()], &ETOKEN_PROGRAM_ID).0
    }

    /// Shuttle PDA: seeds = [owner, mint, shuttle_id_le_bytes]
    pub fn shuttle(owner: &Pubkey, mint: &Pubkey, shuttle_id: u32) -> Pubkey {
        Pubkey::find_program_address(
            &[owner.as_ref(), mint.as_ref(), &shuttle_id.to_le_bytes()],
            &ETOKEN_PROGRAM_ID,
        )
        .0
    }

    /// Transfer Queue PDA: seeds = ["queue", mint, validator]
    pub fn transfer_queue(mint: &Pubkey, validator: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[b"queue", mint.as_ref(), validator.as_ref()],
            &ETOKEN_PROGRAM_ID,
        )
        .0
    }

    /// Rent PDA: seeds = ["rent"]
    pub fn rent_pda() -> Pubkey {
        Pubkey::find_program_address(&[b"rent"], &ETOKEN_PROGRAM_ID).0
    }

    /// Lamports PDA: seeds = ["lamports", payer, destination, salt]
    pub fn lamports_pda(payer: &Pubkey, destination: &Pubkey, salt: &[u8; 32]) -> Pubkey {
        Pubkey::find_program_address(
            &[
                b"lamports",
                payer.as_ref(),
                destination.as_ref(),
                salt.as_ref(),
            ],
            &ETOKEN_PROGRAM_ID,
        )
        .0
    }

    /// Delegation buffer PDA: seeds = ["buffer", delegated_account]
    /// Note: derived under the owner_program (not DELEGATION_PROGRAM_ID)
    pub fn delegation_buffer(delegated_account: &Pubkey, owner_program: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"buffer", delegated_account.as_ref()], owner_program).0
    }

    /// Delegation record PDA: seeds = ["delegation", delegated_account]
    pub fn delegation_record(delegated_account: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[b"delegation", delegated_account.as_ref()],
            &super::DELEGATION_PROGRAM_ID,
        )
        .0
    }

    /// Delegation metadata PDA: seeds = ["delegation-metadata", delegated_account]
    pub fn delegation_metadata(delegated_account: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[b"delegation-metadata", delegated_account.as_ref()],
            &super::DELEGATION_PROGRAM_ID,
        )
        .0
    }
}

// ── Instruction Discriminators (single-byte u32, little-endian) ─────────────

pub mod discriminators {
    pub const INITIALIZE_EPHEMERAL_ATA: [u8; 1] = [0];
    pub const INITIALIZE_GLOBAL_VAULT: [u8; 1] = [1];
    pub const DEPOSIT_SPL_TOKENS: [u8; 1] = [2];
    pub const WITHDRAW_SPL_TOKENS: [u8; 1] = [3];
    pub const DELEGATE_EPHEMERAL_ATA: [u8; 1] = [4];
    pub const UNDELEGATE_EPHEMERAL_ATA: [u8; 1] = [5];
    pub const CREATE_EPHEMERAL_ATA_PERMISSION: [u8; 1] = [6];
    pub const DELEGATE_EPHEMERAL_ATA_PERMISSION: [u8; 1] = [7];
    pub const UNDELEGATE_EPHEMERAL_ATA_PERMISSION: [u8; 1] = [8];
    pub const RESET_EPHEMERAL_ATA_PERMISSION: [u8; 1] = [9];
    pub const CLOSE_EPHEMERAL_ATA: [u8; 1] = [10];
    pub const INITIALIZE_SHUTTLE_EPHEMERAL_ATA: [u8; 1] = [11];
    pub const INITIALIZE_TRANSFER_QUEUE: [u8; 1] = [12];
    pub const DELEGATE_SHUTTLE_EPHEMERAL_ATA: [u8; 1] = [13];
    pub const UNDELEGATE_SHUTTLE_EPHEMERAL_ATA: [u8; 1] = [14];
    pub const MERGE_SHUTTLE_INTO_EPHEMERAL_ATA: [u8; 1] = [15];
    pub const DEPOSIT_AND_QUEUE_TRANSFER: [u8; 1] = [16];
    pub const ENSURE_TRANSFER_QUEUE_CRANK: [u8; 1] = [17];
    pub const DELEGATE_TRANSFER_QUEUE: [u8; 1] = [19];
    pub const SPONSORED_LAMPORTS_TRANSFER: [u8; 1] = [20];
    pub const INITIALIZE_RENT_PDA: [u8; 1] = [23];
    pub const SETUP_AND_DELEGATE_SHUTTLE_WITH_MERGE: [u8; 1] = [24];
    pub const DEPOSIT_AND_DELEGATE_SHUTTLE_WITH_MERGE_AND_PRIVATE_TRANSFER: [u8; 1] = [25];
    pub const WITHDRAW_THROUGH_DELEGATED_SHUTTLE_WITH_MERGE: [u8; 1] = [26];
    pub const ALLOCATE_TRANSFER_QUEUE: [u8; 1] = [27];
}

// ── Core ATA & Vault ────────────────────────────────────────────────────────
pub mod close_ephemeral_ata;
pub mod deposit_spl_tokens;
pub mod initialize_ephemeral_ata;
pub mod initialize_global_vault;
pub mod withdraw_spl_tokens;

// ── Delegation ──────────────────────────────────────────────────────────────
pub mod delegate_ephemeral_ata;
pub mod undelegate_ephemeral_ata;

// ── Permissions ─────────────────────────────────────────────────────────────
pub mod create_ephemeral_ata_permission;
pub mod delegate_ephemeral_ata_permission;
pub mod reset_ephemeral_ata_permission;
pub mod undelegate_ephemeral_ata_permission;

// ── Shuttle Operations ─────────────────────────────────────────────────────
pub mod delegate_shuttle_ephemeral_ata;
pub mod initialize_shuttle_ephemeral_ata;
pub mod merge_shuttle_into_ephemeral_ata;
pub mod undelegate_shuttle_ephemeral_ata;

// ── Transfer Queue ─────────────────────────────────────────────────────────
pub mod allocate_transfer_queue;
pub mod delegate_transfer_queue;
pub mod deposit_and_queue_transfer;
pub mod ensure_transfer_queue_crank;
pub mod initialize_transfer_queue;

// ── Sponsored & Compound Flows ─────────────────────────────────────────────
pub mod initialize_rent_pda;
pub mod setup_and_delegate_shuttle_with_merge;
pub mod sponsored_lamports_transfer;
pub mod withdraw_through_delegated_shuttle_with_merge;
