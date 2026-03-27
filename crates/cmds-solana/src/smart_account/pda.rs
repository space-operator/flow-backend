//! Squads Smart Account PDA derivation functions

use solana_program::pubkey::Pubkey;

use super::PROGRAM_ID;

/// Find the global program config PDA.
///
/// Seeds: `["smart_account", "program_config"]`
pub fn find_program_config() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"smart_account", b"program_config"], &PROGRAM_ID)
}

/// Find the settings PDA for a smart account.
///
/// Seeds: `["smart_account", "settings", account_index_as_u128_le]`
pub fn find_settings(account_index: u128) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"smart_account", b"settings", &account_index.to_le_bytes()],
        &PROGRAM_ID,
    )
}

/// Find the smart account (vault) PDA.
///
/// Seeds: `["smart_account", settings, "smart_account", account_index_u8]`
pub fn find_smart_account(settings: &Pubkey, account_index: u8) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"smart_account",
            settings.as_ref(),
            b"smart_account",
            &[account_index],
        ],
        &PROGRAM_ID,
    )
}

/// Find the transaction PDA.
///
/// Seeds: `["smart_account", settings, "transaction", transaction_index_as_u64_le]`
pub fn find_transaction(settings: &Pubkey, transaction_index: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"smart_account",
            settings.as_ref(),
            b"transaction",
            &transaction_index.to_le_bytes(),
        ],
        &PROGRAM_ID,
    )
}

/// Find the proposal PDA.
///
/// Seeds: `["smart_account", settings, "transaction", transaction_index_as_u64_le, "proposal"]`
pub fn find_proposal(settings: &Pubkey, transaction_index: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"smart_account",
            settings.as_ref(),
            b"transaction",
            &transaction_index.to_le_bytes(),
            b"proposal",
        ],
        &PROGRAM_ID,
    )
}

/// Find the batch transaction PDA.
///
/// Seeds: `["smart_account", settings, "transaction", batch_index_as_u64_le, "batch_transaction", tx_index_as_u32_le]`
pub fn find_batch_transaction(
    settings: &Pubkey,
    batch_index: u64,
    transaction_index: u32,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"smart_account",
            settings.as_ref(),
            b"transaction",
            &batch_index.to_le_bytes(),
            b"batch_transaction",
            &transaction_index.to_le_bytes(),
        ],
        &PROGRAM_ID,
    )
}

/// Find the ephemeral signer PDA.
///
/// Seeds: `["smart_account", transaction, "ephemeral_signer", signer_index_u8]`
pub fn find_ephemeral_signer(transaction: &Pubkey, signer_index: u8) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"smart_account",
            transaction.as_ref(),
            b"ephemeral_signer",
            &[signer_index],
        ],
        &PROGRAM_ID,
    )
}

/// Find the spending limit PDA.
///
/// Seeds: `["smart_account", settings, "spending_limit", seed]`
pub fn find_spending_limit(settings: &Pubkey, seed: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"smart_account",
            settings.as_ref(),
            b"spending_limit",
            seed.as_ref(),
        ],
        &PROGRAM_ID,
    )
}

/// Find the transaction buffer PDA.
///
/// Seeds: `["smart_account", settings, "transaction_buffer", creator, buffer_index_u8]`
pub fn find_transaction_buffer(
    settings: &Pubkey,
    creator: &Pubkey,
    buffer_index: u8,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"smart_account",
            settings.as_ref(),
            b"transaction_buffer",
            creator.as_ref(),
            &[buffer_index],
        ],
        &PROGRAM_ID,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_program_config_deterministic() {
        let (p1, b1) = find_program_config();
        let (p2, b2) = find_program_config();
        assert_eq!(p1, p2);
        assert_eq!(b1, b2);
        assert_ne!(p1, Pubkey::default());
    }

    #[test]
    fn test_find_settings_deterministic() {
        let (p1, _) = find_settings(42);
        let (p2, _) = find_settings(42);
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_different_indices_different_settings() {
        let (p1, _) = find_settings(0);
        let (p2, _) = find_settings(1);
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_find_smart_account() {
        let settings = Pubkey::new_unique();
        let (pda, _) = find_smart_account(&settings, 0);
        assert_ne!(pda, Pubkey::default());
        let (pda2, _) = find_smart_account(&settings, 0);
        assert_eq!(pda, pda2);
    }

    #[test]
    fn test_find_transaction() {
        let settings = Pubkey::new_unique();
        let (pda, _) = find_transaction(&settings, 1);
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn test_find_proposal() {
        let settings = Pubkey::new_unique();
        let (pda, _) = find_proposal(&settings, 1);
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn test_find_batch_transaction() {
        let settings = Pubkey::new_unique();
        let (pda, _) = find_batch_transaction(&settings, 1, 0);
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn test_find_ephemeral_signer() {
        let tx = Pubkey::new_unique();
        let (pda, _) = find_ephemeral_signer(&tx, 0);
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn test_find_spending_limit() {
        let settings = Pubkey::new_unique();
        let seed = Pubkey::new_unique();
        let (pda, _) = find_spending_limit(&settings, &seed);
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn test_find_transaction_buffer() {
        let settings = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let (pda, _) = find_transaction_buffer(&settings, &creator, 0);
        assert_ne!(pda, Pubkey::default());
    }
}
