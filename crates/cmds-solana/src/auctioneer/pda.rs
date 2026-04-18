//! Auctioneer program PDAs.

use super::AUCTIONEER_PROGRAM_ID;
use solana_program::pubkey::Pubkey;

/// The Auctioneer program's own authority PDA for a given AH.
/// Seeds: `[b"auctioneer", auction_house]` under the Auctioneer program.
pub fn find_auctioneer_authority(auction_house: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"auctioneer", auction_house.as_ref()],
        &AUCTIONEER_PROGRAM_ID,
    )
}

/// ListingConfig PDA storing auction params (start_time, end_time, reserve, increment, etc).
/// Seeds: `[b"listing_config", seller_wallet, auction_house, token_account, treasury_mint, token_mint, token_size_le]`
/// under the Auctioneer program. Note: no buyer_price — one listing per (seller, token, size).
pub fn find_listing_config(
    seller: &Pubkey,
    auction_house: &Pubkey,
    token_account: &Pubkey,
    treasury_mint: &Pubkey,
    token_mint: &Pubkey,
    token_size: u64,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"listing_config",
            seller.as_ref(),
            auction_house.as_ref(),
            token_account.as_ref(),
            treasury_mint.as_ref(),
            token_mint.as_ref(),
            &token_size.to_le_bytes(),
        ],
        &AUCTIONEER_PROGRAM_ID,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let ah = Pubkey::new_unique();
        assert_eq!(
            find_auctioneer_authority(&ah),
            find_auctioneer_authority(&ah)
        );
    }

    #[test]
    fn listing_config_varies_with_size() {
        let s = Pubkey::new_unique();
        let ah = Pubkey::new_unique();
        let ta = Pubkey::new_unique();
        let tm = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let a = find_listing_config(&s, &ah, &ta, &tm, &mint, 1).0;
        let b = find_listing_config(&s, &ah, &ta, &tm, &mint, 2).0;
        assert_ne!(a, b);
    }
}
