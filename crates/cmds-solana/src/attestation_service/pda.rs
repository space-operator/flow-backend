//! Attestation Service PDA derivation functions
//!
//! These functions derive Program Derived Addresses (PDAs) for Solana Attestation Service accounts.

use solana_pubkey::Pubkey;

/// Attestation Service Program ID
pub fn attestation_service_program_id() -> Pubkey {
    let program_id_v2 = solana_attestation_service::programs::SOLANA_ATTESTATION_SERVICE_ID;
    Pubkey::new_from_array(program_id_v2.to_bytes())
}

/// Seed for credential PDA
pub const CREDENTIAL_SEED: &[u8] = b"credential";
/// Seed for schema PDA
pub const SCHEMA_SEED: &[u8] = b"schema";
/// Seed for attestation PDA
pub const ATTESTATION_SEED: &[u8] = b"attestation";
/// Seed for schema mint PDA
pub const SCHEMA_MINT_SEED: &[u8] = b"schemaMint";
/// Seed for attestation mint PDA
pub const ATTESTATION_MINT_SEED: &[u8] = b"attestationMint";
/// Seed for event authority PDA
pub const EVENT_AUTHORITY_SEED: &[u8] = b"__event_authority";
/// Seed for SAS authority PDA
pub const SAS_AUTHORITY_SEED: &[u8] = b"sas";

/// Find a credential PDA
///
/// Seeds: ["credential", authority_pubkey, name]
pub fn find_credential(authority: &Pubkey, name: &str) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[CREDENTIAL_SEED, authority.as_ref(), name.as_bytes()],
        &attestation_service_program_id(),
    )
}

/// Find a schema PDA
///
/// Seeds: ["schema", credential_pda, name, version_byte]
pub fn find_schema(credential: &Pubkey, name: &str, version: u8) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            SCHEMA_SEED,
            credential.as_ref(),
            name.as_bytes(),
            &[version],
        ],
        &attestation_service_program_id(),
    )
}

/// Find an attestation PDA
///
/// Seeds: ["attestation", credential_pda, schema_pda, nonce_pubkey]
pub fn find_attestation(credential: &Pubkey, schema: &Pubkey, nonce: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            ATTESTATION_SEED,
            credential.as_ref(),
            schema.as_ref(),
            nonce.as_ref(),
        ],
        &attestation_service_program_id(),
    )
}

/// Find a schema mint PDA
///
/// Seeds: ["schema_mint", schema_pda]
pub fn find_schema_mint(schema: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[SCHEMA_MINT_SEED, schema.as_ref()],
        &attestation_service_program_id(),
    )
}

/// Find an attestation mint PDA
///
/// Seeds: ["attestation_mint", attestation_pda]
pub fn find_attestation_mint(attestation: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[ATTESTATION_MINT_SEED, attestation.as_ref()],
        &attestation_service_program_id(),
    )
}

/// Find the event authority PDA
///
/// Seeds: ["__event_authority"]
pub fn find_event_authority() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[EVENT_AUTHORITY_SEED], &attestation_service_program_id())
}

/// Derive the SAS authority PDA
///
/// Seeds: ["sas"]
pub fn derive_sas_authority_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[SAS_AUTHORITY_SEED], &attestation_service_program_id())
}

/// Token-2022 program ID
pub fn token_2022_program_id() -> Pubkey {
    // TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
    Pubkey::new_from_array(spl_token_2022::ID.to_bytes())
}

/// ATA program ID
pub fn ata_program_id() -> Pubkey {
    // ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
    Pubkey::new_from_array(spl_associated_token_account::ID.to_bytes())
}

/// Find a recipient token account (ATA) for Token-2022
///
/// Uses the official SPL ATA derivation for Token-2022
pub fn find_recipient_token_account(recipient: &Pubkey, attestation_mint: &Pubkey) -> (Pubkey, u8) {
    // ATA derivation: PDA([wallet, token_program, mint], ata_program)
    // The SPL function expects solana_program::pubkey::Pubkey, which uses different byte layout
    // So we do the derivation manually using the same seeds
    Pubkey::find_program_address(
        &[
            recipient.as_ref(),
            token_2022_program_id().as_ref(),
            attestation_mint.as_ref(),
        ],
        &ata_program_id(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_credential() {
        let authority = Pubkey::new_unique();
        let (pda, bump) = find_credential(&authority, "test_credential");
        assert_ne!(pda, Pubkey::default());
        let _ = bump;
    }

    #[test]
    fn test_find_schema() {
        let credential = Pubkey::new_unique();
        let (pda, bump) = find_schema(&credential, "test_schema", 1);
        assert_ne!(pda, Pubkey::default());
        let _ = bump;
    }

    #[test]
    fn test_find_attestation() {
        let credential = Pubkey::new_unique();
        let schema = Pubkey::new_unique();
        let nonce = Pubkey::new_unique();
        let (pda, bump) = find_attestation(&credential, &schema, &nonce);
        assert_ne!(pda, Pubkey::default());
        let _ = bump;
    }

    #[test]
    fn test_find_schema_mint() {
        let schema = Pubkey::new_unique();
        let (pda, bump) = find_schema_mint(&schema);
        assert_ne!(pda, Pubkey::default());
        let _ = bump;
    }

    #[test]
    fn test_find_attestation_mint() {
        let attestation = Pubkey::new_unique();
        let (pda, bump) = find_attestation_mint(&attestation);
        assert_ne!(pda, Pubkey::default());
        let _ = bump;
    }

    #[test]
    fn test_find_event_authority() {
        let (pda, bump) = find_event_authority();
        assert_ne!(pda, Pubkey::default());
        let _ = bump;
    }

    #[test]
    fn test_derive_sas_authority_address() {
        let (pda, bump) = derive_sas_authority_address();
        assert_ne!(pda, Pubkey::default());
        let _ = bump;
    }

    #[test]
    fn test_find_recipient_token_account() {
        let recipient = Pubkey::new_unique();
        let attestation_mint = Pubkey::new_unique();
        let (pda, bump) = find_recipient_token_account(&recipient, &attestation_mint);
        assert_ne!(pda, Pubkey::default());
        let _ = bump;
    }

    #[test]
    fn test_program_ids() {
        // Verify Token-2022 program ID matches expected
        let token_2022 = token_2022_program_id();
        // TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
        assert_eq!(token_2022.to_bytes(), spl_token_2022::ID.to_bytes());

        // Verify ATA program ID matches expected
        let ata = ata_program_id();
        // ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
        assert_eq!(ata.to_bytes(), spl_associated_token_account::ID.to_bytes());
    }

    #[test]
    fn test_derivations_are_deterministic() {
        let authority = Pubkey::new_unique();
        let name = "test_credential";
        let (pda1, _) = find_credential(&authority, name);
        let (pda2, _) = find_credential(&authority, name);
        assert_eq!(pda1, pda2);
    }
}
