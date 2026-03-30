//! Rewards program PDA derivation functions

use solana_program::pubkey::Pubkey;

use super::REWARDS_PROGRAM_ID;

/// Find the direct distribution PDA.
///
/// Seeds: `["direct_distribution", mint, authority, seeds]`
pub fn find_direct_distribution(mint: &Pubkey, authority: &Pubkey, seeds: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"direct_distribution",
            mint.as_ref(),
            authority.as_ref(),
            seeds.as_ref(),
        ],
        &REWARDS_PROGRAM_ID,
    )
}

/// Find the direct recipient PDA.
///
/// Seeds: `["direct_recipient", distribution, recipient]`
pub fn find_direct_recipient(distribution: &Pubkey, recipient: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"direct_recipient",
            distribution.as_ref(),
            recipient.as_ref(),
        ],
        &REWARDS_PROGRAM_ID,
    )
}

/// Find the merkle distribution PDA.
///
/// Seeds: `["merkle_distribution", mint, authority, seeds]`
pub fn find_merkle_distribution(mint: &Pubkey, authority: &Pubkey, seeds: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"merkle_distribution",
            mint.as_ref(),
            authority.as_ref(),
            seeds.as_ref(),
        ],
        &REWARDS_PROGRAM_ID,
    )
}

/// Find the merkle claim PDA.
///
/// Seeds: `["merkle_claim", distribution_or_pool, claimant]`
pub fn find_merkle_claim(distribution_or_pool: &Pubkey, claimant: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"merkle_claim",
            distribution_or_pool.as_ref(),
            claimant.as_ref(),
        ],
        &REWARDS_PROGRAM_ID,
    )
}

/// Find the continuous reward pool PDA.
///
/// Seeds: `["reward_pool", reward_mint, tracked_mint, authority, seed]`
pub fn find_reward_pool(
    reward_mint: &Pubkey,
    tracked_mint: &Pubkey,
    authority: &Pubkey,
    seed: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"reward_pool",
            reward_mint.as_ref(),
            tracked_mint.as_ref(),
            authority.as_ref(),
            seed.as_ref(),
        ],
        &REWARDS_PROGRAM_ID,
    )
}

/// Find the user reward account PDA.
///
/// Seeds: `["user_reward", reward_pool, user]`
pub fn find_user_reward_account(reward_pool: &Pubkey, user: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"user_reward", reward_pool.as_ref(), user.as_ref()],
        &REWARDS_PROGRAM_ID,
    )
}

/// Find the revocation marker PDA.
///
/// Seeds: `["revocation", distribution_or_pool, user]`
pub fn find_revocation_marker(distribution_or_pool: &Pubkey, user: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"revocation", distribution_or_pool.as_ref(), user.as_ref()],
        &REWARDS_PROGRAM_ID,
    )
}

/// Find the event authority PDA.
///
/// Seeds: `["__event_authority"]`
pub fn find_event_authority() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"__event_authority"], &REWARDS_PROGRAM_ID)
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
    fn test_find_direct_distribution_deterministic() {
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let seeds = Pubkey::new_unique();
        let (p1, b1) = find_direct_distribution(&mint, &authority, &seeds);
        let (p2, b2) = find_direct_distribution(&mint, &authority, &seeds);
        assert_eq!(p1, p2);
        assert_eq!(b1, b2);
        assert_ne!(p1, Pubkey::default());
    }

    #[test]
    fn test_different_seeds_different_distributions() {
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let s1 = Pubkey::new_unique();
        let s2 = Pubkey::new_unique();
        let (p1, _) = find_direct_distribution(&mint, &authority, &s1);
        let (p2, _) = find_direct_distribution(&mint, &authority, &s2);
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_find_direct_recipient() {
        let distribution = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let (pda, _) = find_direct_recipient(&distribution, &recipient);
        assert_ne!(pda, Pubkey::default());
        let (pda2, _) = find_direct_recipient(&distribution, &recipient);
        assert_eq!(pda, pda2);
    }

    #[test]
    fn test_find_merkle_distribution() {
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let seeds = Pubkey::new_unique();
        let (pda, _) = find_merkle_distribution(&mint, &authority, &seeds);
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn test_find_merkle_claim() {
        let distribution = Pubkey::new_unique();
        let claimant = Pubkey::new_unique();
        let (pda, _) = find_merkle_claim(&distribution, &claimant);
        assert_ne!(pda, Pubkey::default());
        let (pda2, _) = find_merkle_claim(&distribution, &claimant);
        assert_eq!(pda, pda2);
    }

    #[test]
    fn test_find_reward_pool() {
        let reward_mint = Pubkey::new_unique();
        let tracked_mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let seed = Pubkey::new_unique();
        let (pda, _) = find_reward_pool(&reward_mint, &tracked_mint, &authority, &seed);
        assert_ne!(pda, Pubkey::default());
        let (pda2, _) = find_reward_pool(&reward_mint, &tracked_mint, &authority, &seed);
        assert_eq!(pda, pda2);
    }

    #[test]
    fn test_find_user_reward_account() {
        let pool = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let (pda, _) = find_user_reward_account(&pool, &user);
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn test_find_revocation_marker() {
        let dist = Pubkey::new_unique();
        let user = Pubkey::new_unique();
        let (pda, _) = find_revocation_marker(&dist, &user);
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
        let token_program = solana_program::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
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
