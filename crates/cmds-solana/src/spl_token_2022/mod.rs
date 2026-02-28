//! Token-2022 (Token Extensions) — Space Operator nodes.
//!
//! Program ID: `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`
//! SDK: spl-token-2022-interface v2.1.0 (solana-* v3 compatible)

use crate::prelude::*;
use spl_associated_token_account_interface::address::get_associated_token_address_with_program_id;

/// Derive the Associated Token Account address for a Token-2022 mint.
pub fn derive_ata(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id(wallet, mint, &spl_token_2022_interface::ID)
}

// ── Core token operations ───────────────────────────────────────────────
pub mod approve_checked;
pub mod burn_checked;
pub mod close_account;
pub mod create_native_mint;
pub mod freeze_account;
pub mod get_account_data_size;
pub mod initialize_account;
pub mod initialize_mint;
pub mod mint_to_checked;
pub mod reallocate;
pub mod revoke;
pub mod set_authority;
pub mod sync_native;
pub mod thaw_account;
pub mod transfer_checked;
pub mod withdraw_excess_lamports;

// ── Transfer Fee extension ──────────────────────────────────────────────
pub mod transfer_fee;

// ── Pointer extensions ──────────────────────────────────────────────────
pub mod group_member_pointer;
pub mod group_pointer;
pub mod metadata_pointer;

// ── Access control extensions ───────────────────────────────────────────
pub mod cpi_guard;
pub mod memo_transfer;
pub mod pausable;

// ── Interest & UI extensions ────────────────────────────────────────────
pub mod default_account_state;
pub mod interest_bearing;
pub mod scaled_ui_amount;
pub mod transfer_hook;
