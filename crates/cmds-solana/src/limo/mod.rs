//! Kamino Limo (Limit Orders) nodes for Space Operator
//!
//! On-chain Solana instruction nodes for the Kamino Limo program.
//! Program ID: LiMoM9rMhrdYrfzUCxQppvxCSG1FcrUK9G8uLq4A1GF


// limo - Space Operator nodes for Kamino Limo (Limit Orders)
//
// Program ID: `LiMoM9rMhrdYrfzUCxQppvxCSG1FcrUK9G8uLq4A1GF`
// Repository: https://github.com/Kamino-Finance/limo
//
// Limit order matching with flash take orders, vaults,
// host tips, and swap balance assertions.

use crate::prelude::*;

// =============================================================================
// Program Constants
// =============================================================================

/// Limo (Limit Orders) Program ID
pub const LIMO_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("LiMoM9rMhrdYrfzUCxQppvxCSG1FcrUK9G8uLq4A1GF");

/// System Program ID
pub const SYSTEM_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("11111111111111111111111111111111");

/// Token Program ID
pub const TOKEN_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

// =============================================================================
// Anchor Discriminator
// =============================================================================

/// Compute Anchor 8-byte instruction discriminator: sha256("global:{name}")[..8]
pub fn anchor_discriminator(name: &str) -> [u8; 8] {
    let preimage = format!("global:{}", name);
    let hash = solana_program::hash::hash(preimage.as_bytes());
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash.to_bytes()[..8]);
    disc
}

// =============================================================================
// ATA Derivation
// =============================================================================

/// Derive the Associated Token Address for a given owner, mint, and token program.
/// Supports both SPL Token and Token-2022 programs.
pub fn derive_ata(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    spl_associated_token_account_interface::address::get_associated_token_address_with_program_id(
        owner,
        mint,
        token_program,
    )
}

// =============================================================================
// Node Modules - Global Config (Admin)
// =============================================================================

pub mod initialize_global_config;
pub mod update_global_config;
pub mod update_global_config_admin;

// =============================================================================
// Node Modules - Vault Management
// =============================================================================

pub mod initialize_vault;

// =============================================================================
// Node Modules - Order Operations
// =============================================================================

pub mod create_order;
pub mod update_order;
pub mod close_order_and_claim_tip;
pub mod take_order;

// =============================================================================
// Node Modules - Flash Take Orders
// =============================================================================

pub mod flash_take_order_start;
pub mod flash_take_order_end;

// =============================================================================
// Node Modules - Tips & Balances
// =============================================================================

pub mod withdraw_host_tip;
pub mod log_user_swap_balances_start;
pub mod log_user_swap_balances_end;
pub mod assert_user_swap_balances_start;
pub mod assert_user_swap_balances_end;
