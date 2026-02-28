//! Kamino Vaults (kvault) nodes for Space Operator
//!
//! On-chain Solana instruction nodes for the Kamino Vault program.
//! Program ID: KvauGMspG5k6rtzrqqn7WNn3oZdyKqLKwK2XWQ8FLjd


// kvault - Space Operator nodes for Kamino Vaults
//
// Program ID: `KvauGMspG5k6rtzrqqn7WNn3oZdyKqLKwK2XWQ8FLjd`
// Repository: https://github.com/Kamino-Finance/kvault
//
// Earn vaults with reserve allocation, deposits, withdrawals,
// and share management.

use crate::prelude::*;

// =============================================================================
// Program Constants
// =============================================================================

/// Kamino Vault Program ID
pub const KVAULT_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("KvauGMspG5k6rtzrqqn7WNn3oZdyKqLKwK2XWQ8FLjd");

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

/// Derive standard SPL Associated Token Account
pub fn derive_ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    spl_associated_token_account_interface::address::get_associated_token_address_with_program_id(
        owner,
        mint,
        &TOKEN_PROGRAM_ID,
    )
}

// =============================================================================
// Node Modules - Global Config (Admin)
// =============================================================================

pub mod init_global_config;
pub mod update_global_config;
pub mod update_global_config_admin;

// =============================================================================
// Node Modules - Vault Management (Admin)
// =============================================================================

pub mod init_vault;
pub mod update_vault_config;
pub mod update_admin;
pub mod update_reserve_allocation;
pub mod remove_allocation;
pub mod add_update_whitelisted_reserve;

// =============================================================================
// Node Modules - User Operations
// =============================================================================

pub mod deposit;
pub mod buy;
pub mod withdraw;
pub mod sell;
pub mod invest;
pub mod withdraw_from_available;

// =============================================================================
// Node Modules - Shares Metadata
// =============================================================================

pub mod initialize_shares_metadata;
pub mod update_shares_metadata;

// =============================================================================
// Node Modules - Fee Management (Admin)
// =============================================================================

pub mod withdraw_pending_fees;
pub mod give_up_pending_fees;
