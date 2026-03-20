//! Solana v2 ↔ v3 type conversion helpers.
//!
//! Several crates in this workspace (attestation-service, swig, tuktuk,
//! light-compressed-token-sdk) depend on `solana-program` v2. Our workspace
//! uses `solana-pubkey` v3 and `solana-instruction` v3. Since these are
//! separate Rust types, we need zero-copy bridging via `to_bytes()`.

/// Convert workspace `solana-pubkey` v3 `Pubkey` to `solana-program` v2 `Pubkey`.
#[inline]
pub fn to_pubkey_v2(pk: &solana_pubkey::Pubkey) -> solana_program_v2::pubkey::Pubkey {
    solana_program_v2::pubkey::Pubkey::new_from_array(pk.to_bytes())
}

/// Convert `solana-program` v2 `Instruction` to workspace `solana-instruction` v3.
#[inline]
pub fn to_instruction_v3(
    ix: solana_program_v2::instruction::Instruction,
) -> solana_instruction::Instruction {
    solana_instruction::Instruction {
        program_id: solana_pubkey::Pubkey::new_from_array(ix.program_id.to_bytes()),
        accounts: ix
            .accounts
            .into_iter()
            .map(|a| solana_instruction::AccountMeta {
                pubkey: solana_pubkey::Pubkey::new_from_array(a.pubkey.to_bytes()),
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            })
            .collect(),
        data: ix.data,
    }
}
