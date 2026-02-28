//! Meteora DAMM v2 (Dynamic AMM v2) CP-AMM nodes for Space Operator
//!
//! On-chain Solana instruction nodes for the Meteora DAMM v2 program.
//! Program ID: cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG


// damm_v2 - Space Operator nodes for Meteora DAMM v2 (Dynamic AMM v2)
//
// Program ID: `cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG`
// Repository: https://github.com/MeteoraAg/damm-v2
//
// Constant Product AMM with NFT-based LP positions, vesting/locking,
// dual rewards, operator permissions, and dynamic fees.

use crate::prelude::*;

// =============================================================================
// Program Constants
// =============================================================================

/// DAMM v2 CP-AMM Program ID
pub const CP_AMM_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");

/// System Program ID
pub const SYSTEM_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("11111111111111111111111111111111");

/// Token Program ID
pub const TOKEN_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Token-2022 Program ID
pub const TOKEN_2022_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// Associated Token Account Program ID
pub const ATA_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

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

/// Derive event authority PDA for Anchor event CPI
pub fn derive_event_authority() -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], &CP_AMM_PROGRAM_ID).0
}

// =============================================================================
// PDA Seeds & Derivation
// =============================================================================

pub const POOL_PREFIX: &[u8] = b"pool";
pub const CUSTOMIZABLE_POOL_PREFIX: &[u8] = b"customizable_pool";
pub const POSITION_PREFIX: &[u8] = b"position";
pub const POSITION_NFT_ACCOUNT_PREFIX: &[u8] = b"position_nft_account";
pub const TOKEN_VAULT_PREFIX: &[u8] = b"token_vault";
pub const CONFIG_PREFIX: &[u8] = b"config";
pub const OPERATOR_PREFIX: &[u8] = b"operator";
pub const TOKEN_BADGE_PREFIX: &[u8] = b"token_badge";
pub const REWARD_VAULT_PREFIX: &[u8] = b"reward_vault";

/// Pool authority PDA (global constant, not per-pool)
pub fn derive_pool_authority() -> Pubkey {
    Pubkey::find_program_address(&[b"pool_authority"], &CP_AMM_PROGRAM_ID).0
}

/// Derive config PDA from index
pub fn derive_config(index: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[CONFIG_PREFIX, &index.to_le_bytes()],
        &CP_AMM_PROGRAM_ID,
    ).0
}

/// Derive pool PDA from config, token_a_mint, token_b_mint, and creator
pub fn derive_pool(config: &Pubkey, token_a_mint: &Pubkey, token_b_mint: &Pubkey, creator: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[POOL_PREFIX, config.as_ref(), token_a_mint.as_ref(), token_b_mint.as_ref(), creator.as_ref()],
        &CP_AMM_PROGRAM_ID,
    ).0
}

/// Derive token vault PDA from pool and token_mint
pub fn derive_token_vault(pool: &Pubkey, token_mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[TOKEN_VAULT_PREFIX, pool.as_ref(), token_mint.as_ref()],
        &CP_AMM_PROGRAM_ID,
    ).0
}

/// Derive position PDA from pool and position_nft_mint
pub fn derive_position(pool: &Pubkey, position_nft_mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[POSITION_PREFIX, pool.as_ref(), position_nft_mint.as_ref()],
        &CP_AMM_PROGRAM_ID,
    ).0
}

/// Derive operator PDA
pub fn derive_operator(operator_pubkey: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[OPERATOR_PREFIX, operator_pubkey.as_ref()],
        &CP_AMM_PROGRAM_ID,
    ).0
}

/// Derive token badge PDA
pub fn derive_token_badge(token_mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[TOKEN_BADGE_PREFIX, token_mint.as_ref()],
        &CP_AMM_PROGRAM_ID,
    ).0
}

/// Derive reward vault PDA
pub fn derive_reward_vault(pool: &Pubkey, reward_mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[REWARD_VAULT_PREFIX, pool.as_ref(), reward_mint.as_ref()],
        &CP_AMM_PROGRAM_ID,
    ).0
}

// =============================================================================
// Node Modules - Pool Initialization
// =============================================================================

pub mod initialize_pool;
pub mod initialize_customizable_pool;
pub mod initialize_pool_with_dynamic_config;

// =============================================================================
// Node Modules - Liquidity Management
// =============================================================================

pub mod create_position;
pub mod add_liquidity;
pub mod remove_liquidity;
pub mod remove_all_liquidity;
pub mod close_position;

// =============================================================================
// Node Modules - Trading
// =============================================================================

pub mod swap;
pub mod swap2;

// =============================================================================
// Node Modules - Position Fees & Rewards
// =============================================================================

pub mod claim_position_fee;
pub mod claim_reward;

// =============================================================================
// Node Modules - Position Locking & Vesting
// =============================================================================

pub mod lock_position;
pub mod lock_inner_position;
pub mod permanent_lock_position;
pub mod refresh_vesting;
pub mod split_position;

// =============================================================================
// Node Modules - Config Management (Operator)
// =============================================================================

pub mod create_config;
pub mod create_dynamic_config;
pub mod close_config;

// =============================================================================
// Node Modules - Token Badge Management (Operator)
// =============================================================================

pub mod create_token_badge;
pub mod close_token_badge;

// =============================================================================
// Node Modules - Reward Management
// =============================================================================

pub mod initialize_reward;
pub mod fund_reward;
pub mod withdraw_ineligible_reward;
pub mod update_reward_funder;
pub mod update_reward_duration;

// =============================================================================
// Node Modules - Pool Administration (Operator)
// =============================================================================

pub mod set_pool_status;
pub mod update_pool_fees;
pub mod fix_pool_fee_params;
pub mod fix_config_fee_params;

// =============================================================================
// Node Modules - Fee Collection (Operator/Partner)
// =============================================================================

pub mod claim_protocol_fee;
pub mod zap_protocol_fee;
pub mod claim_partner_fee;

// =============================================================================
// Node Modules - Admin
// =============================================================================

pub mod create_operator_account;
pub mod close_operator_account;
