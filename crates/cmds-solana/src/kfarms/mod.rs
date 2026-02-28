//! Kamino Farms (kfarms) nodes for Space Operator
//!
//! On-chain Solana instruction nodes for the Kamino Farms program.
//! Program ID: FarmsPZpWu9i7Kky8tPN37rs2TpmMrAZrC7S7vJa91Hr


// kfarms - Space Operator nodes for Kamino Farms
//
// Program ID: `FarmsPZpWu9i7Kky8tPN37rs2TpmMrAZrC7S7vJa91Hr`
// Repository: https://github.com/Kamino-Finance/kfarms
//
// Staking and reward distribution for Kamino DeFi positions.

use crate::prelude::*;

// =============================================================================
// Program Constants
// =============================================================================

/// Kamino Farms Program ID
pub const KFARMS_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("FarmsPZpWu9i7Kky8tPN37rs2TpmMrAZrC7S7vJa91Hr");

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
// PDA Seeds & Derivation
// =============================================================================

/// Derive user state PDA from farm and owner
pub fn derive_user_state(farm: &Pubkey, owner: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"user", farm.as_ref(), owner.as_ref()],
        &KFARMS_PROGRAM_ID,
    ).0
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

pub mod initialize_global_config;
pub mod update_global_config;
pub mod update_global_config_admin;

// =============================================================================
// Node Modules - Farm Management (Admin)
// =============================================================================

pub mod initialize_farm;
pub mod initialize_farm_delegated;
pub mod update_farm_config;
pub mod update_farm_admin;
pub mod transfer_ownership;

// =============================================================================
// Node Modules - Reward Management (Admin)
// =============================================================================

pub mod initialize_reward;
pub mod add_rewards;
pub mod reward_user_once;

// =============================================================================
// Node Modules - User Operations
// =============================================================================

pub mod initialize_user;
pub mod stake;
pub mod set_stake_delegated;
pub mod harvest_reward;
pub mod unstake;

// =============================================================================
// Node Modules - Refresh & Maintenance
// =============================================================================

pub mod refresh_farm;
pub mod refresh_user_state;
pub mod withdraw_unstaked_deposits;

// =============================================================================
// Node Modules - Vault & Treasury (Admin)
// =============================================================================

pub mod deposit_to_farm_vault;
pub mod withdraw_from_farm_vault;
pub mod withdraw_treasury;
pub mod withdraw_slashed_amount;
