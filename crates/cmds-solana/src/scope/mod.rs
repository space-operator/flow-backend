//! Kamino Scope Oracle nodes for Space Operator
//!
//! On-chain Solana instruction nodes for the Kamino Scope Oracle program.
//! Program ID: HFn8GnPADiny6XqUoWE8uRPPxb29ikn4yTnUa9MF2fWJ


// scope - Space Operator nodes for Kamino Scope Oracle
//
// Program ID: `HFn8GnPADiny6XqUoWE8uRPPxb29ikn4yTnUa9MF2fWJ`
// Repository: https://github.com/Kamino-Finance/scope
//
// Oracle aggregator for Solana DeFi price feeds.

use crate::prelude::*;

// =============================================================================
// Program Constants
// =============================================================================

/// Scope Oracle Program ID
pub const SCOPE_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("HFn8GnPADiny6XqUoWE8uRPPxb29ikn4yTnUa9MF2fWJ");

/// System Program ID
pub const SYSTEM_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("11111111111111111111111111111111");

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
// Node Modules - Oracle Operations
// =============================================================================

pub mod initialize;
pub mod refresh_one_price;
pub mod refresh_price_list;
pub mod update_mapping;
