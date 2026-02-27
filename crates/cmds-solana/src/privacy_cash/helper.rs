use borsh::BorshSerialize;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;

use super::pda;

pub use crate::utils::anchor_discriminator;

/// Build a complete Anchor instruction: discriminator + borsh-serialized args.
pub fn build_instruction(
    instruction_name: &str,
    accounts: Vec<AccountMeta>,
    args_data: Vec<u8>,
) -> Instruction {
    crate::utils::build_anchor_instruction(pda::program_id(), instruction_name, accounts, args_data)
}

/// Build an instruction with no args (discriminator only).
pub fn build_instruction_no_args(
    instruction_name: &str,
    accounts: Vec<AccountMeta>,
) -> Instruction {
    build_instruction(instruction_name, accounts, vec![])
}

// ── Borsh-serializable arg structs matching the on-chain program ──

/// Proof struct matching the on-chain Groth16 proof format.
#[derive(BorshSerialize, Clone, Debug)]
pub struct Proof {
    pub proof_a: [u8; 64],
    pub proof_b: [u8; 128],
    pub proof_c: [u8; 64],
    pub root: [u8; 32],
    pub public_amount: [u8; 32],
    pub ext_data_hash: [u8; 32],
    pub input_nullifiers: [[u8; 32]; 2],
    pub output_commitments: [[u8; 32]; 2],
}

/// Minified extended data sent as instruction args.
#[derive(BorshSerialize, Clone, Debug)]
pub struct ExtDataMinified {
    pub ext_amount: i64,
    pub fee: u64,
}

/// System program ID.
pub fn system_program() -> Pubkey {
    "11111111111111111111111111111111".parse().unwrap()
}

/// SPL Token program ID.
pub fn token_program() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        .parse()
        .unwrap()
}

/// Associated Token Account program ID.
pub fn ata_program() -> Pubkey {
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        .parse()
        .unwrap()
}

/// Derive the Associated Token Account address.
pub fn find_ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program().as_ref(), mint.as_ref()],
        &ata_program(),
    )
    .0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchor_discriminator_length() {
        let disc = anchor_discriminator("initialize");
        assert_eq!(disc.len(), 8);
    }

    #[test]
    fn test_anchor_discriminator_deterministic() {
        let disc1 = anchor_discriminator("initialize");
        let disc2 = anchor_discriminator("initialize");
        assert_eq!(disc1, disc2);
    }

    #[test]
    fn test_anchor_discriminator_unique_per_instruction() {
        let d_init = anchor_discriminator("initialize");
        let d_transact = anchor_discriminator("transact");
        let d_update = anchor_discriminator("update_deposit_limit");
        assert_ne!(d_init, d_transact);
        assert_ne!(d_init, d_update);
        assert_ne!(d_transact, d_update);
    }

    #[test]
    fn test_proof_serialization_size() {
        let proof = Proof {
            proof_a: [0u8; 64],
            proof_b: [0u8; 128],
            proof_c: [0u8; 64],
            root: [0u8; 32],
            public_amount: [0u8; 32],
            ext_data_hash: [0u8; 32],
            input_nullifiers: [[0u8; 32]; 2],
            output_commitments: [[0u8; 32]; 2],
        };
        let mut data = Vec::new();
        proof.serialize(&mut data).unwrap();
        // 64 + 128 + 64 + 32 + 32 + 32 + 64 + 64 = 480
        assert_eq!(data.len(), 480, "Proof borsh size must be 480 bytes");
    }

    #[test]
    fn test_ext_data_minified_serialization_size() {
        let ext = ExtDataMinified {
            ext_amount: -100_000,
            fee: 500,
        };
        let mut data = Vec::new();
        ext.serialize(&mut data).unwrap();
        // i64 (8) + u64 (8) = 16
        assert_eq!(
            data.len(),
            16,
            "ExtDataMinified borsh size must be 16 bytes"
        );
    }

    #[test]
    fn test_build_instruction_has_discriminator_prefix() {
        let accounts = vec![AccountMeta::new_readonly(system_program(), false)];
        let args = vec![1u8, 2, 3, 4];
        let ix = build_instruction("initialize", accounts, args.clone());

        let disc = anchor_discriminator("initialize");
        assert_eq!(
            &ix.data[..8],
            &disc,
            "instruction data must start with discriminator"
        );
        assert_eq!(&ix.data[8..], &args, "instruction data must end with args");
        assert_eq!(ix.program_id, pda::program_id());
    }

    #[test]
    fn test_build_instruction_no_args_is_discriminator_only() {
        let accounts = vec![];
        let ix = build_instruction_no_args("initialize", accounts);
        assert_eq!(
            ix.data.len(),
            8,
            "no-args instruction data is just the 8-byte discriminator"
        );
    }

    #[test]
    fn test_well_known_program_ids() {
        assert_eq!(
            system_program().to_string(),
            "11111111111111111111111111111111"
        );
        assert_eq!(
            token_program().to_string(),
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        );
        assert_eq!(
            ata_program().to_string(),
            "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        );
    }

    #[test]
    fn test_find_ata_deterministic() {
        let owner: Pubkey = "97rSMQUukMDjA7PYErccyx7ZxbHvSDaeXp2ig5BwSrTf"
            .parse()
            .unwrap();
        let mint: Pubkey = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
            .parse()
            .unwrap();
        let ata1 = find_ata(&owner, &mint);
        let ata2 = find_ata(&owner, &mint);
        assert_eq!(ata1, ata2);
        assert_ne!(ata1, Pubkey::default());
    }
}
