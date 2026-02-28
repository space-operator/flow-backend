//! Kamino Merkle Distributor nodes for Space Operator
//!
//! On-chain Solana instruction nodes for the Kamino Merkle Distributor program.
//! Program ID: KdisqEcXbXKaTrBFqeDLhMmBvymLTwj9GmhDcdJyGat


// merkle_distributor - Space Operator nodes for Kamino Merkle Distributor
//
// Program ID: `KdisqEcXbXKaTrBFqeDLhMmBvymLTwj9GmhDcdJyGat`
// Repository: https://github.com/Kamino-Finance/distributor
//
// Merkle-based token distribution for airdrops with locked/unlocked
// vesting, clawback, and claim management.

use crate::prelude::*;

// =============================================================================
// Program Constants
// =============================================================================

/// Merkle Distributor Program ID
pub const MERKLE_DISTRIBUTOR_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("KdisqEcXbXKaTrBFqeDLhMmBvymLTwj9GmhDcdJyGat");

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

/// Derive claim status PDA
pub fn derive_claim_status(index: u64, distributor: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"ClaimStatus", &index.to_le_bytes(), distributor.as_ref()],
        &MERKLE_DISTRIBUTOR_PROGRAM_ID,
    ).0
}

// =============================================================================
// Node Modules - Distributor Setup (Admin)
// =============================================================================

pub mod new_distributor;
pub mod close_distributor;
pub mod set_admin;

// =============================================================================
// Node Modules - Configuration (Admin)
// =============================================================================

pub mod set_enable_slot;
pub mod set_clawback_receiver;

// =============================================================================
// Node Modules - Claims (User)
// =============================================================================

pub mod new_claim;
pub mod claim_locked;
pub mod close_claim_status;

// =============================================================================
// Node Modules - Clawback (Admin)
// =============================================================================

pub mod clawback;
