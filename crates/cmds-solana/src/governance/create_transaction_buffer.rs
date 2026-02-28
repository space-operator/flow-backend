use solana_program::instruction::AccountMeta;
use solana_sdk_ids::system_program;

use super::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID, get_proposal_transaction_buffer_address};

const NAME: &str = "create_transaction_buffer";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        flow_lib::node_definition!("/governance/create_transaction_buffer.jsonc");
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    #[serde(with = "value::pubkey")]
    pub governance: Pubkey,
    #[serde(with = "value::pubkey")]
    pub proposal: Pubkey,
    #[serde(with = "value::pubkey")]
    pub token_owner_record: Pubkey,
    pub governance_authority: Wallet,
    pub buffer_index: u8,
    /// SHA-256 hash of the final buffer content (exactly 32 bytes)
    pub final_buffer_hash: Vec<u8>,
    pub final_buffer_size: u16,
    /// Initial buffer content chunk
    pub buffer: Vec<u8>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

/// v3.1.2 account layout:
/// 0. `[]` Governance account
/// 1. `[writable]` Proposal account
/// 2. `[]` TokenOwnerRecord account
/// 3. `[signer]` Governance Authority
/// 4. `[writable]` ProposalTransactionBuffer account (PDA)
/// 5. `[writable, signer]` Payer
/// 6. `[]` System program
#[allow(clippy::too_many_arguments)]
pub fn create_transaction_buffer(
    program_id: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    payer: &Pubkey,
    buffer_index: u8,
    final_buffer_hash: [u8; 32],
    final_buffer_size: u16,
    buffer: Vec<u8>,
) -> (Instruction, Pubkey) {
    let proposal_transaction_buffer_address = get_proposal_transaction_buffer_address(
        program_id,
        proposal,
        governance_authority,
        &[buffer_index],
    );

    let accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(proposal_transaction_buffer_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let data = GovernanceInstruction::CreateTransactionBuffer {
        buffer_index,
        final_buffer_hash,
        final_buffer_size,
        buffer,
    };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instruction, proposal_transaction_buffer_address)
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let final_buffer_hash: [u8; 32] = input
        .final_buffer_hash
        .try_into()
        .map_err(|_| CommandError::msg("final_buffer_hash must be exactly 32 bytes"))?;

    let (ix, proposal_transaction_buffer_address) = create_transaction_buffer(
        &program_id,
        &input.governance,
        &input.proposal,
        &input.token_owner_record,
        &input.governance_authority.pubkey(),
        &input.fee_payer.pubkey(),
        input.buffer_index,
        final_buffer_hash,
        input.final_buffer_size,
        input.buffer,
    );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.governance_authority].into(),
        instructions: [ix].into(),
    };

    let signature = ctx
        .execute(
            instructions,
            value::map!(
                "proposal_transaction_buffer_address" => proposal_transaction_buffer_address,
            ),
        )
        .await?
        .signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::super::SPL_GOVERNANCE_ID;
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_instruction_builder() {
        let governance = Pubkey::new_unique();
        let proposal = Pubkey::new_unique();
        let token_owner_record = Pubkey::new_unique();
        let governance_authority = Pubkey::new_unique();
        let payer = Pubkey::new_unique();

        let (ix, _addr) = create_transaction_buffer(
            &SPL_GOVERNANCE_ID,
            &governance,
            &proposal,
            &token_owner_record,
            &governance_authority,
            &payer,
            0u8,
            [0u8; 32],
            0u16,
            vec![1, 2, 3],
        );

        assert_eq!(ix.program_id, SPL_GOVERNANCE_ID);
        assert!(!ix.data.is_empty());
        assert!(ix.accounts.len() >= 7);
    }
}
