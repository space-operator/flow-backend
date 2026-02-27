//! Jupiter Locker PDA derivation functions

use solana_program::pubkey::Pubkey;

use super::JUP_LOCK_PROGRAM_ID;

/// Find the escrow PDA from a base keypair.
///
/// Seeds: `["escrow", base_pubkey]`
pub fn find_escrow(base: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"escrow", base.as_ref()], &JUP_LOCK_PROGRAM_ID)
}

/// Find the escrow metadata PDA from an escrow address.
///
/// Seeds: `["escrow_metadata", escrow_pubkey]`
pub fn find_escrow_metadata(escrow: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"escrow_metadata", escrow.as_ref()], &JUP_LOCK_PROGRAM_ID)
}

/// Find the Anchor event authority PDA.
///
/// Seeds: `["__event_authority"]`
pub fn find_event_authority() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"__event_authority"], &JUP_LOCK_PROGRAM_ID)
}

/// Find the Associated Token Account for a given wallet, mint, and token program.
///
/// Seeds: `[wallet, token_program, mint]` under the ATA program.
pub fn find_ata(wallet: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[wallet.as_ref(), token_program.as_ref(), mint.as_ref()],
        &spl_associated_token_account_interface::program::ID,
    )
}

#[cfg(test)]
mod tests {
    use solana_program::pubkey;

    use super::*;

    #[test]
    fn test_find_escrow() {
        let base = Pubkey::new_unique();
        let (pda, _bump) = find_escrow(&base);
        assert_ne!(pda, Pubkey::default());
        // Deterministic
        let (pda2, bump2) = find_escrow(&base);
        assert_eq!(pda, pda2);
        assert_eq!(_bump, bump2);
    }

    #[test]
    fn test_find_escrow_metadata() {
        let escrow = Pubkey::new_unique();
        let (pda, _bump) = find_escrow_metadata(&escrow);
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn test_find_event_authority_deterministic() {
        let (pda1, _) = find_event_authority();
        let (pda2, _) = find_event_authority();
        assert_eq!(pda1, pda2);
    }

    #[test]
    fn test_find_ata() {
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_program = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let (ata, _bump) = find_ata(&wallet, &mint, &token_program);
        assert_ne!(ata, Pubkey::default());
        // Deterministic
        let (ata2, _) = find_ata(&wallet, &mint, &token_program);
        assert_eq!(ata, ata2);
    }

    #[test]
    fn test_different_bases_different_escrows() {
        let base1 = Pubkey::new_unique();
        let base2 = Pubkey::new_unique();
        let (pda1, _) = find_escrow(&base1);
        let (pda2, _) = find_escrow(&base2);
        assert_ne!(
            pda1, pda2,
            "Different bases must produce different escrow PDAs"
        );
    }

    #[test]
    fn test_escrow_and_metadata_are_different() {
        let base = Pubkey::new_unique();
        let (escrow, _) = find_escrow(&base);
        let (metadata, _) = find_escrow_metadata(&escrow);
        assert_ne!(escrow, metadata, "Escrow and its metadata PDA must differ");
    }

    #[test]
    fn test_different_mints_different_atas() {
        let wallet = Pubkey::new_unique();
        let mint1 = Pubkey::new_unique();
        let mint2 = Pubkey::new_unique();
        let token_program = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let (ata1, _) = find_ata(&wallet, &mint1, &token_program);
        let (ata2, _) = find_ata(&wallet, &mint2, &token_program);
        assert_ne!(ata1, ata2, "Different mints must produce different ATAs");
    }
}
