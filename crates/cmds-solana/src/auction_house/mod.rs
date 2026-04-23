//! Metaplex Auction House program Space Operator nodes
//!
//! Program ID: `hausS13jsjafwWwGqZTUQRmWyvyxn9EQpqMwV1PBBmk`
//! Source: https://github.com/solana-foundation/anchor/tree/master/tests/auction-house
//!
//! Direct instruction construction (no SDK crate dependency).
//! Anchor program: instructions use 8-byte sha256("global:<name>")[..8] discriminators.

use crate::prelude::*;
use solana_program::pubkey;

pub mod buy;
pub mod cancel;
pub mod create_auction_house;
pub mod delegate_auctioneer;
pub mod deposit;
pub mod execute_sale;
pub mod pda;
pub mod sell;
pub mod update_auction_house;
pub mod withdraw;
pub mod withdraw_from_fee;
pub mod withdraw_from_treasury;

/// Auction House program ID
pub const AUCTION_HOUSE_PROGRAM_ID: Pubkey = pubkey!("hausS13jsjafwWwGqZTUQRmWyvyxn9EQpqMwV1PBBmk");

/// SPL Token program ID (used as `token_program` in most instructions)
pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// SPL Associated Token Account program ID
pub const ATA_PROGRAM_ID: Pubkey = pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

/// Metaplex Token Metadata program ID
pub const TOKEN_METADATA_PROGRAM_ID: Pubkey =
    pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

/// Wrapped-SOL (native) mint — used when treasury pays in SOL
pub const NATIVE_SOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

/// True if the treasury mint is wrapped-SOL (native). Determines whether payment/receipt
/// accounts are plain SOL wallets (native) or associated token accounts (SPL).
pub fn is_native_mint(mint: &Pubkey) -> bool {
    mint == &NATIVE_SOL_MINT
}

/// Canonical payment/receipt account for a given wallet+mint: returns the wallet itself
/// when the mint is native SOL, otherwise the associated token account for (wallet, mint).
pub fn payment_account_for(wallet: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    if is_native_mint(mint) {
        *wallet
    } else {
        pda::find_ata(wallet, mint, token_program).0
    }
}

/// Serde default: SPL Token program
pub fn default_token_program() -> Pubkey {
    TOKEN_PROGRAM_ID
}

/// Serde default: wrapped-SOL mint
pub fn default_native_mint() -> Pubkey {
    NATIVE_SOL_MINT
}

// Anchor 8-byte instruction discriminators: sha256("global:<snake_name>")[..8]
pub const DISC_CREATE_AUCTION_HOUSE: [u8; 8] = [0xdd, 0x42, 0xf2, 0x9f, 0xf9, 0xce, 0x86, 0xf1];
pub const DISC_DEPOSIT: [u8; 8] = [0xf2, 0x23, 0xc6, 0x89, 0x52, 0xe1, 0xf2, 0xb6];
pub const DISC_WITHDRAW: [u8; 8] = [0xb7, 0x12, 0x46, 0x9c, 0x94, 0x6d, 0xa1, 0x22];
pub const DISC_SELL: [u8; 8] = [0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad];
pub const DISC_BUY: [u8; 8] = [0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea];
pub const DISC_EXECUTE_SALE: [u8; 8] = [0x25, 0x4a, 0xd9, 0x9d, 0x4f, 0x31, 0x23, 0x06];
pub const DISC_CANCEL: [u8; 8] = [0xe8, 0xdb, 0xdf, 0x29, 0xdb, 0xec, 0xdc, 0xbe];
pub const DISC_WITHDRAW_FROM_FEE: [u8; 8] = [0xb3, 0xd0, 0xbe, 0x9a, 0x20, 0xb3, 0x13, 0x3b];
pub const DISC_WITHDRAW_FROM_TREASURY: [u8; 8] = [0x00, 0xa4, 0x56, 0x4c, 0x38, 0x48, 0x0c, 0xaa];
pub const DISC_UPDATE_AUCTION_HOUSE: [u8; 8] = [0x54, 0xd7, 0x02, 0xac, 0xf1, 0x00, 0xf5, 0xdb];
pub const DISC_DELEGATE_AUCTIONEER: [u8; 8] = [0x6a, 0xb2, 0x0c, 0x7a, 0x4a, 0xad, 0xfb, 0xde];

/// Build an Auction House instruction: 8-byte discriminator + borsh args.
pub fn build_auction_house_instruction(
    discriminator: [u8; 8],
    accounts: Vec<solana_program::instruction::AccountMeta>,
    args_data: Vec<u8>,
) -> Instruction {
    let mut data = Vec::with_capacity(8 + args_data.len());
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&args_data);
    Instruction {
        program_id: AUCTION_HOUSE_PROGRAM_ID,
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
        assert_eq!(
            DISC_CREATE_AUCTION_HOUSE,
            anchor_disc("create_auction_house")
        );
        assert_eq!(DISC_DEPOSIT, anchor_disc("deposit"));
        assert_eq!(DISC_WITHDRAW, anchor_disc("withdraw"));
        assert_eq!(DISC_SELL, anchor_disc("sell"));
        assert_eq!(DISC_BUY, anchor_disc("buy"));
        assert_eq!(DISC_EXECUTE_SALE, anchor_disc("execute_sale"));
        assert_eq!(DISC_CANCEL, anchor_disc("cancel"));
        assert_eq!(DISC_WITHDRAW_FROM_FEE, anchor_disc("withdraw_from_fee"));
        assert_eq!(
            DISC_WITHDRAW_FROM_TREASURY,
            anchor_disc("withdraw_from_treasury")
        );
        assert_eq!(
            DISC_UPDATE_AUCTION_HOUSE,
            anchor_disc("update_auction_house")
        );
        assert_eq!(DISC_DELEGATE_AUCTIONEER, anchor_disc("delegate_auctioneer"));
    }

    #[test]
    fn test_build_instruction() {
        let ix =
            build_auction_house_instruction(DISC_DEPOSIT, vec![], 42u64.to_le_bytes().to_vec());
        assert_eq!(ix.program_id, AUCTION_HOUSE_PROGRAM_ID);
        assert_eq!(ix.data[..8], DISC_DEPOSIT);
        assert_eq!(ix.data.len(), 16);
    }
}
