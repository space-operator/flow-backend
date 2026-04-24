//! Metaplex Auctioneer program Space Operator nodes
//!
//! Program ID: `neer8g6yJq2mQM6KbnViEDAD4gr3gRZyMMf4F2p3MEh`
//! Source: https://github.com/metaplex-foundation/metaplex-program-library/tree/master/auctioneer
//!
//! Auctioneer is a delegate program layered on top of `mpl-auction-house`: it wraps AH's
//! `auctioneer_*` instructions and enforces English-auction rules (timed window,
//! reserve price, min bid increment, anti-sniping time extension, optional high-bid cancel).
//!
//! Flow to use:
//!   1. `auction_house_create` (from the auction_house module) with `requires_sign_off: true`.
//!   2. Compute the auctioneer_authority PDA via `pda::find_auctioneer_authority`.
//!   3. Call `auction_house_delegate_auctioneer` to register the auctioneer on the AH side.
//!   4. Call `auctioneer_authorize` to create the auctioneer-side state.
//!   5. `auctioneer_sell` (with auction params) → bidders `auctioneer_deposit` + `auctioneer_buy`
//!      → `auctioneer_execute_sale` (after end_time) → losing bidders `auctioneer_withdraw`.

use crate::prelude::*;
use solana_program::pubkey;

pub mod authorize;
pub mod buy;
pub mod cancel;
pub mod deposit;
pub mod execute_sale;
pub mod pda;
pub mod sell;
pub mod withdraw;

/// Auctioneer program ID
pub const AUCTIONEER_PROGRAM_ID: Pubkey = pubkey!("neer8g6yJq2mQM6KbnViEDAD4gr3gRZyMMf4F2p3MEh");

/// Sentinel buyer_price used in the seller's AH trade_state seed when listing via Auctioneer.
/// Bids can land at any price, so the listing side doesn't commit to one.
pub const AUCTIONEER_BUYER_PRICE: u64 = u64::MAX;

// Anchor 8-byte instruction discriminators: sha256("global:<snake_name>")[..8]
pub const DISC_AUTHORIZE: [u8; 8] = [0xad, 0xc1, 0x66, 0xd2, 0xdb, 0x89, 0x71, 0x78];
pub const DISC_SELL: [u8; 8] = [0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad];
pub const DISC_BUY: [u8; 8] = [0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea];
pub const DISC_DEPOSIT: [u8; 8] = [0xf2, 0x23, 0xc6, 0x89, 0x52, 0xe1, 0xf2, 0xb6];
pub const DISC_WITHDRAW: [u8; 8] = [0xb7, 0x12, 0x46, 0x9c, 0x94, 0x6d, 0xa1, 0x22];
pub const DISC_CANCEL: [u8; 8] = [0xe8, 0xdb, 0xdf, 0x29, 0xdb, 0xec, 0xdc, 0xbe];
pub const DISC_EXECUTE_SALE: [u8; 8] = [0x25, 0x4a, 0xd9, 0x9d, 0x4f, 0x31, 0x23, 0x06];

pub fn build_auctioneer_instruction(
    discriminator: [u8; 8],
    accounts: Vec<solana_program::instruction::AccountMeta>,
    args_data: Vec<u8>,
) -> Instruction {
    let mut data = Vec::with_capacity(8 + args_data.len());
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&args_data);
    Instruction {
        program_id: AUCTIONEER_PROGRAM_ID,
        accounts,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn anchor_disc(name: &str) -> [u8; 8] {
        let mut h = Sha256::new();
        h.update(format!("global:{name}").as_bytes());
        let out = h.finalize();
        let mut b = [0u8; 8];
        b.copy_from_slice(&out[..8]);
        b
    }

    #[test]
    fn test_discriminators() {
        assert_eq!(DISC_AUTHORIZE, anchor_disc("authorize"));
        assert_eq!(DISC_SELL, anchor_disc("sell"));
        assert_eq!(DISC_BUY, anchor_disc("buy"));
        assert_eq!(DISC_DEPOSIT, anchor_disc("deposit"));
        assert_eq!(DISC_WITHDRAW, anchor_disc("withdraw"));
        assert_eq!(DISC_CANCEL, anchor_disc("cancel"));
        assert_eq!(DISC_EXECUTE_SALE, anchor_disc("execute_sale"));
    }
}
