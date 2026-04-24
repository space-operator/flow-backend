//! Auction House program PDA derivations.

use solana_program::pubkey::Pubkey;

use super::{AUCTION_HOUSE_PROGRAM_ID, TOKEN_METADATA_PROGRAM_ID};

pub const PREFIX: &[u8] = b"auction_house";
pub const FEE_PAYER: &[u8] = b"fee_payer";
pub const TREASURY: &[u8] = b"treasury";
pub const SIGNER: &[u8] = b"signer";
/// Zero byte sequence used for "free" trade states (no buyer_price).
pub const ZERO: [u8; 8] = [0u8; 8];

/// `[PREFIX, authority, treasury_mint]`
pub fn find_auction_house(authority: &Pubkey, treasury_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[PREFIX, authority.as_ref(), treasury_mint.as_ref()],
        &AUCTION_HOUSE_PROGRAM_ID,
    )
}

/// `[PREFIX, auction_house, FEE_PAYER]`
pub fn find_auction_house_fee_account(auction_house: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[PREFIX, auction_house.as_ref(), FEE_PAYER],
        &AUCTION_HOUSE_PROGRAM_ID,
    )
}

/// `[PREFIX, auction_house, TREASURY]`
pub fn find_auction_house_treasury(auction_house: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[PREFIX, auction_house.as_ref(), TREASURY],
        &AUCTION_HOUSE_PROGRAM_ID,
    )
}

/// `[PREFIX, auction_house, wallet]`
pub fn find_escrow_payment_account(auction_house: &Pubkey, wallet: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[PREFIX, auction_house.as_ref(), wallet.as_ref()],
        &AUCTION_HOUSE_PROGRAM_ID,
    )
}

/// `[PREFIX, wallet, auction_house, token_account, treasury_mint, token_mint, buyer_price_le, token_size_le]`
pub fn find_trade_state(
    wallet: &Pubkey,
    auction_house: &Pubkey,
    token_account: &Pubkey,
    treasury_mint: &Pubkey,
    token_mint: &Pubkey,
    buyer_price: u64,
    token_size: u64,
) -> (Pubkey, u8) {
    let buyer_price_le = buyer_price.to_le_bytes();
    let token_size_le = token_size.to_le_bytes();
    Pubkey::find_program_address(
        &[
            PREFIX,
            wallet.as_ref(),
            auction_house.as_ref(),
            token_account.as_ref(),
            treasury_mint.as_ref(),
            token_mint.as_ref(),
            &buyer_price_le,
            &token_size_le,
        ],
        &AUCTION_HOUSE_PROGRAM_ID,
    )
}

/// Free trade state: same seeds as `find_trade_state` but with ZERO for buyer_price.
pub fn find_free_trade_state(
    wallet: &Pubkey,
    auction_house: &Pubkey,
    token_account: &Pubkey,
    treasury_mint: &Pubkey,
    token_mint: &Pubkey,
    token_size: u64,
) -> (Pubkey, u8) {
    let token_size_le = token_size.to_le_bytes();
    Pubkey::find_program_address(
        &[
            PREFIX,
            wallet.as_ref(),
            auction_house.as_ref(),
            token_account.as_ref(),
            treasury_mint.as_ref(),
            token_mint.as_ref(),
            &ZERO,
            &token_size_le,
        ],
        &AUCTION_HOUSE_PROGRAM_ID,
    )
}

/// AH-side Auctioneer PDA that stores delegated scopes.
/// Seeds: `[b"auctioneer", auction_house, auctioneer_authority]` under the AH program.
pub fn find_ah_auctioneer_pda(
    auction_house: &Pubkey,
    auctioneer_authority: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"auctioneer",
            auction_house.as_ref(),
            auctioneer_authority.as_ref(),
        ],
        &AUCTION_HOUSE_PROGRAM_ID,
    )
}

/// `[PREFIX, SIGNER]`
pub fn find_program_as_signer() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PREFIX, SIGNER], &AUCTION_HOUSE_PROGRAM_ID)
}

/// Metaplex Token Metadata PDA: `["metadata", token_metadata_program, mint]`.
pub fn find_metadata(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"metadata",
            TOKEN_METADATA_PROGRAM_ID.as_ref(),
            mint.as_ref(),
        ],
        &TOKEN_METADATA_PROGRAM_ID,
    )
}

/// Associated Token Account: `[wallet, token_program, mint]` under the ATA program.
pub fn find_ata(wallet: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[wallet.as_ref(), token_program.as_ref(), mint.as_ref()],
        &spl_associated_token_account_interface::program::ID,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let a = Pubkey::new_unique();
        let m = Pubkey::new_unique();
        assert_eq!(find_auction_house(&a, &m), find_auction_house(&a, &m));
        let (ah, _) = find_auction_house(&a, &m);
        assert_eq!(
            find_auction_house_fee_account(&ah),
            find_auction_house_fee_account(&ah)
        );
        assert_eq!(
            find_auction_house_treasury(&ah),
            find_auction_house_treasury(&ah)
        );
        assert_eq!(
            find_escrow_payment_account(&ah, &a),
            find_escrow_payment_account(&ah, &a)
        );
        assert_eq!(find_program_as_signer(), find_program_as_signer());
    }

    #[test]
    fn seed_sensitivity() {
        let a1 = Pubkey::new_unique();
        let a2 = Pubkey::new_unique();
        let m = Pubkey::new_unique();
        assert_ne!(find_auction_house(&a1, &m).0, find_auction_house(&a2, &m).0);
    }

    #[test]
    fn trade_state_vs_free_differ() {
        let w = Pubkey::new_unique();
        let ah = Pubkey::new_unique();
        let ta = Pubkey::new_unique();
        let tm = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let a = find_trade_state(&w, &ah, &ta, &tm, &mint, 100, 1).0;
        let b = find_free_trade_state(&w, &ah, &ta, &tm, &mint, 1).0;
        assert_ne!(a, b);
    }
}
