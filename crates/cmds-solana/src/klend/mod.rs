//! Kamino Lending (klend) nodes for Space Operator
//!
//! On-chain Solana instruction nodes for the Kamino Lending program.
//! Program ID: KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD


// klend - Space Operator nodes for Kamino Lending
//
// Program ID: `KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD`
// Repository: https://github.com/Kamino-Finance/klend
//
// Lending protocol with deposits, borrows, obligations, liquidations,
// flash loans, referrals, and elevation groups.

use crate::prelude::*;

// =============================================================================
// Program Constants
// =============================================================================

/// Kamino Lending Program ID
pub const KLEND_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD");

/// System Program ID
pub const SYSTEM_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("11111111111111111111111111111111");

/// Token Program ID
pub const TOKEN_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

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

// =============================================================================
// PDA Seeds & Derivation
// =============================================================================

/// Derive lending market authority PDA
pub fn derive_lending_market_authority(lending_market: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"lma", lending_market.as_ref()],
        &KLEND_PROGRAM_ID,
    ).0
}

/// Derive obligation PDA from lending market, owner, and seed accounts
pub fn derive_obligation(lending_market: &Pubkey, owner: &Pubkey, seed1: &Pubkey, seed2: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"obligation", lending_market.as_ref(), owner.as_ref(), seed1.as_ref(), seed2.as_ref()],
        &KLEND_PROGRAM_ID,
    ).0
}

/// Derive user metadata PDA
pub fn derive_user_metadata(owner: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"user_meta", owner.as_ref()],
        &KLEND_PROGRAM_ID,
    ).0
}

/// Derive referrer token state PDA
pub fn derive_referrer_token_state(referrer: &Pubkey, reserve: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"referrer_acc", referrer.as_ref(), reserve.as_ref()],
        &KLEND_PROGRAM_ID,
    ).0
}

/// Derive referrer state and short URL PDA
pub fn derive_referrer_state(short_url: &str) -> Pubkey {
    Pubkey::find_program_address(
        &[b"referrer_short_url", short_url.as_bytes()],
        &KLEND_PROGRAM_ID,
    ).0
}

// =============================================================================
// Node Modules - Market Setup (Admin)
// =============================================================================

pub mod init_lending_market;
pub mod update_lending_market;
pub mod update_lending_market_owner;

// =============================================================================
// Node Modules - Reserve Management (Admin)
// =============================================================================

pub mod init_reserve;
pub mod init_farms_for_reserve;
pub mod update_single_reserve_config;
pub mod update_entire_reserve_config;
pub mod refresh_reserve;

// =============================================================================
// Node Modules - Deposits & Redemptions
// =============================================================================

pub mod deposit_reserve_liquidity;
pub mod redeem_reserve_collateral;

// =============================================================================
// Node Modules - Obligations
// =============================================================================

pub mod init_obligation;
pub mod init_obligation_farms_for_reserve;
pub mod refresh_obligation_farms_for_reserve;
pub mod refresh_obligation;
pub mod deposit_obligation_collateral;
pub mod withdraw_obligation_collateral;
pub mod borrow_obligation_liquidity;
pub mod repay_obligation_liquidity;

// =============================================================================
// Node Modules - Combined Operations
// =============================================================================

pub mod deposit_reserve_liquidity_and_obligation_collateral;
pub mod withdraw_obligation_collateral_and_redeem_reserve_collateral;

// =============================================================================
// Node Modules - Liquidation
// =============================================================================

pub mod liquidate_obligation_and_redeem_reserve_collateral;

// =============================================================================
// Node Modules - Flash Loans
// =============================================================================

pub mod flash_borrow_reserve_liquidity;
pub mod flash_repay_reserve_liquidity;

// =============================================================================
// Node Modules - Fee & Risk Management
// =============================================================================

pub mod redeem_fees;
pub mod withdraw_protocol_fee;
pub mod socialize_loss;

// =============================================================================
// Node Modules - Elevation Groups
// =============================================================================

pub mod request_elevation_group;

// =============================================================================
// Node Modules - Referrals & User Metadata
// =============================================================================

pub mod init_referrer_token_state;
pub mod init_user_metadata;
pub mod withdraw_referrer_fees;
pub mod init_referrer_state_and_short_url;
pub mod delete_referrer_state_and_short_url;
