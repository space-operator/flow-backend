//! Metaplex Agent program nodes for Space Operator
//!
//! On-chain instruction nodes for the MPL Agent program suite:
//! - Identity (1DREGFgysWYxLnRnKQnwrxnJQeSMk2HmGaC6whw2B2p)
//! - Reputation (REPREG5c1gPHuHukEyANpksLdHFaJCiTrm6zJgNhRZR)
//! - Validation (VALREGY66A9ieJfFUNs5GrxFTy498KUoSU7TbmSePQi)
//! - Tools (TLREGni9ZEyGC3vnPZtqUh95xQ8oPqJSvNjvB7FGK8S)
//!
//! Repository: https://github.com/metaplex-foundation/mpl-agent

pub mod delegate_execution_v1;
pub mod register_executive_v1;
pub mod register_identity_v1;
pub mod register_reputation_v1;
pub mod register_validation_v1;
pub mod revoke_execution_v1;
pub mod set_agent_token_v1;

// Re-export v2↔v3 conversion helpers
pub use crate::solana_v2_compat::{to_instruction_v3, to_pubkey_v2};

use solana_pubkey::Pubkey;

// Program IDs (v3 types for PDA derivation)
pub fn identity_program_id() -> Pubkey {
    Pubkey::new_from_array(mpl_agent_identity::ID.to_bytes())
}

pub fn reputation_program_id() -> Pubkey {
    Pubkey::new_from_array(mpl_agent_reputation::ID.to_bytes())
}

pub fn validation_program_id() -> Pubkey {
    Pubkey::new_from_array(mpl_agent_validation::ID.to_bytes())
}

pub fn tools_program_id() -> Pubkey {
    Pubkey::new_from_array(mpl_agent_tools::ID.to_bytes())
}

// PDA derivation functions

/// Find the agent identity PDA for a given Core asset.
/// Seeds: ["agent_identity", asset]
pub fn find_agent_identity_pda(asset: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"agent_identity", asset.as_ref()], &identity_program_id())
}

/// Find the agent reputation PDA for a given Core asset.
/// Seeds: ["agent_reputation", asset]
pub fn find_agent_reputation_pda(asset: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"agent_reputation", asset.as_ref()],
        &reputation_program_id(),
    )
}

/// Find the agent validation PDA for a given Core asset.
/// Seeds: ["agent_validation", asset]
pub fn find_agent_validation_pda(asset: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"agent_validation", asset.as_ref()],
        &validation_program_id(),
    )
}

/// Find the executive profile PDA for a given authority.
/// Seeds: ["executive_profile", authority]
pub fn find_executive_profile_pda(authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"executive_profile", authority.as_ref()],
        &tools_program_id(),
    )
}

/// Find the execution delegate record PDA.
/// Seeds: ["execution_delegate_record", executive_profile, agent_asset]
pub fn find_execution_delegate_record_pda(
    executive_profile: &Pubkey,
    agent_asset: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"execution_delegate_record",
            executive_profile.as_ref(),
            agent_asset.as_ref(),
        ],
        &tools_program_id(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_ids() {
        let identity = identity_program_id();
        assert_ne!(identity, Pubkey::default());

        let reputation = reputation_program_id();
        assert_ne!(reputation, Pubkey::default());

        let validation = validation_program_id();
        assert_ne!(validation, Pubkey::default());

        let tools = tools_program_id();
        assert_ne!(tools, Pubkey::default());
    }

    #[test]
    fn test_pdas_are_deterministic() {
        let asset = Pubkey::new_unique();
        let (pda1, _) = find_agent_identity_pda(&asset);
        let (pda2, _) = find_agent_identity_pda(&asset);
        assert_eq!(pda1, pda2);
    }

    #[test]
    fn test_find_agent_identity_pda() {
        let asset = Pubkey::new_unique();
        let (pda, bump) = find_agent_identity_pda(&asset);
        assert_ne!(pda, Pubkey::default());
        let _ = bump;
    }

    #[test]
    fn test_find_agent_reputation_pda() {
        let asset = Pubkey::new_unique();
        let (pda, bump) = find_agent_reputation_pda(&asset);
        assert_ne!(pda, Pubkey::default());
        let _ = bump;
    }

    #[test]
    fn test_find_agent_validation_pda() {
        let asset = Pubkey::new_unique();
        let (pda, bump) = find_agent_validation_pda(&asset);
        assert_ne!(pda, Pubkey::default());
        let _ = bump;
    }

    #[test]
    fn test_find_executive_profile_pda() {
        let authority = Pubkey::new_unique();
        let (pda, bump) = find_executive_profile_pda(&authority);
        assert_ne!(pda, Pubkey::default());
        let _ = bump;
    }

    #[test]
    fn test_find_execution_delegate_record_pda() {
        let exec_profile = Pubkey::new_unique();
        let agent_asset = Pubkey::new_unique();
        let (pda, bump) = find_execution_delegate_record_pda(&exec_profile, &agent_asset);
        assert_ne!(pda, Pubkey::default());
        let _ = bump;
    }
}
