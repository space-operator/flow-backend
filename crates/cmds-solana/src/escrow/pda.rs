//! Escrow program PDA derivation functions

use solana_program::pubkey::Pubkey;

use super::ESCROW_PROGRAM_ID;

/// Find the escrow PDA.
///
/// Seeds: `["escrow", escrow_seed]`
pub fn find_escrow(escrow_seed: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"escrow", escrow_seed.as_ref()], &ESCROW_PROGRAM_ID)
}

/// Find the receipt PDA.
///
/// Seeds: `["receipt", escrow, depositor, mint, receipt_seed]`
pub fn find_receipt(
    escrow: &Pubkey,
    depositor: &Pubkey,
    mint: &Pubkey,
    receipt_seed: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"receipt",
            escrow.as_ref(),
            depositor.as_ref(),
            mint.as_ref(),
            receipt_seed.as_ref(),
        ],
        &ESCROW_PROGRAM_ID,
    )
}

/// Find the allowed-mint PDA.
///
/// Seeds: `["allowed_mint", escrow, mint]`
pub fn find_allowed_mint(escrow: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"allowed_mint", escrow.as_ref(), mint.as_ref()],
        &ESCROW_PROGRAM_ID,
    )
}

/// Find the extensions PDA.
///
/// Seeds: `["extensions", escrow]`
pub fn find_extensions(escrow: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"extensions", escrow.as_ref()], &ESCROW_PROGRAM_ID)
}

/// Find the Anchor-style event authority PDA.
///
/// Seeds: `["__event_authority"]`
pub fn find_event_authority() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"__event_authority"], &ESCROW_PROGRAM_ID)
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
    use super::*;

    #[test]
    fn test_find_escrow_deterministic() {
        let seed = Pubkey::new_unique();
        let (pda1, b1) = find_escrow(&seed);
        let (pda2, b2) = find_escrow(&seed);
        assert_eq!(pda1, pda2);
        assert_eq!(b1, b2);
        assert_ne!(pda1, Pubkey::default());
    }

    #[test]
    fn test_different_seeds_different_escrows() {
        let s1 = Pubkey::new_unique();
        let s2 = Pubkey::new_unique();
        let (p1, _) = find_escrow(&s1);
        let (p2, _) = find_escrow(&s2);
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_find_receipt() {
        let escrow = Pubkey::new_unique();
        let depositor = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let receipt_seed = Pubkey::new_unique();
        let (pda, _) = find_receipt(&escrow, &depositor, &mint, &receipt_seed);
        assert_ne!(pda, Pubkey::default());
        // Deterministic
        let (pda2, _) = find_receipt(&escrow, &depositor, &mint, &receipt_seed);
        assert_eq!(pda, pda2);
    }

    #[test]
    fn test_find_allowed_mint() {
        let escrow = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let (pda, _) = find_allowed_mint(&escrow, &mint);
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn test_find_extensions() {
        let escrow = Pubkey::new_unique();
        let (pda, _) = find_extensions(&escrow);
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn test_find_event_authority_deterministic() {
        let (p1, _) = find_event_authority();
        let (p2, _) = find_event_authority();
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_find_ata() {
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_program =
            solana_program::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let (ata, _) = find_ata(&wallet, &mint, &token_program);
        assert_ne!(ata, Pubkey::default());
        let (ata2, _) = find_ata(&wallet, &mint, &token_program);
        assert_eq!(ata, ata2);
    }

    #[test]
    fn test_different_mints_different_atas() {
        let wallet = Pubkey::new_unique();
        let m1 = Pubkey::new_unique();
        let m2 = Pubkey::new_unique();
        let tp = solana_program::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let (a1, _) = find_ata(&wallet, &m1, &tp);
        let (a2, _) = find_ata(&wallet, &m2, &tp);
        assert_ne!(a1, a2);
    }
}
