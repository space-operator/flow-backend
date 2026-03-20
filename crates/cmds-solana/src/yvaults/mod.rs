//! Kamino Liquidity / YVaults nodes for Space Operator
//!
//! On-chain Solana instruction nodes for the Kamino YVaults program.
//! Automated LP strategy management with deposits, withdrawals, and rebalancing.

// yvaults - Space Operator nodes for Kamino Liquidity / YVaults
//
// Repository: https://github.com/Kamino-Finance/kvault
//
// Automated LP strategy management with concentrated liquidity,
// deposits, withdrawals, fee collection, and rebalancing.

use crate::prelude::*;

// =============================================================================
// Program Constants
// =============================================================================

/// Kamino Liquidity (YVaults) Program ID
pub const YVAULTS_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("6LtLpnUFNByNXLyCoK9wA2MykKAmQNZKBdY8s47dehDc");

/// System Program ID
pub const SYSTEM_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("11111111111111111111111111111111");

/// Token Program ID
pub const TOKEN_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Associated Token Account Program ID
pub const ATA_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

// =============================================================================
// PDA & ATA Derivation
// =============================================================================

/// Derive standard SPL Associated Token Account
pub fn derive_ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    spl_associated_token_account_interface::address::get_associated_token_address_with_program_id(
        owner,
        mint,
        &TOKEN_PROGRAM_ID,
    )
}

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
// Node Modules - Strategy Initialization (Admin)
// =============================================================================

pub mod add_kamino_rewards;
pub mod initialize_kamino_reward;
pub mod initialize_strategy;

// =============================================================================
// Node Modules - Global Config (Admin)
// =============================================================================

pub mod initialize_global_config;
pub mod update_global_config;

// =============================================================================
// Node Modules - Collateral & Metadata (Admin)
// =============================================================================

pub mod initialize_collateral_info;
pub mod initialize_shares_metadata;
pub mod update_collateral_info;
pub mod update_shares_metadata;

// =============================================================================
// Node Modules - Strategy Config (Admin)
// =============================================================================

pub mod update_reward_mapping;
pub mod update_strategy_config;
pub mod update_treasury_fee_vault;

// =============================================================================
// Node Modules - User Operations
// =============================================================================

pub mod deposit;
pub mod deposit_and_invest;
pub mod invest;
pub mod open_liquidity_position;
pub mod sign_terms;
pub mod withdraw;

// =============================================================================
// Node Modules - Fee & Reward Collection
// =============================================================================

pub mod collect_fees_and_rewards;
pub mod swap_rewards;

// =============================================================================
// Node Modules - Rebalancing & Swaps
// =============================================================================

pub mod flash_swap_uneven_vaults_end;
pub mod flash_swap_uneven_vaults_start;
pub mod orca_swap;
pub mod swap_uneven_vaults;

// =============================================================================
// Node Modules - Admin Operations
// =============================================================================

pub mod change_pool;
pub mod close_program_account;
pub mod emergency_swap;
pub mod executive_withdraw;
pub mod withdraw_from_treasury;
