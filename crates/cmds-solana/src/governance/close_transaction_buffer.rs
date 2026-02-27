use solana_program::instruction::AccountMeta;

use super::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID, get_proposal_transaction_buffer_address};

const NAME: &str = "close_transaction_buffer";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        flow_lib::node_definition!("/governance/close_transaction_buffer.jsonc");
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
    pub beneficiary: Wallet,
    pub buffer_index: u8,
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
/// 1. `[]` Proposal account
/// 2. `[]` TokenOwnerRecord account
/// 3. `[signer]` Governance Authority
/// 4. `[writable]` ProposalTransactionBuffer account (PDA)
/// 5. `[writable, signer]` Beneficiary account (receives rent)
#[allow(clippy::too_many_arguments)]
pub fn close_transaction_buffer(
    program_id: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    beneficiary: &Pubkey,
    buffer_index: u8,
) -> (Instruction, Pubkey) {
    let proposal_transaction_buffer_address = get_proposal_transaction_buffer_address(
        program_id,
        proposal,
        governance_authority,
        &[buffer_index],
    );

    let accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new_readonly(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(proposal_transaction_buffer_address, false),
        AccountMeta::new(*beneficiary, true),
    ];

    let data = GovernanceInstruction::CloseTransactionBuffer { buffer_index };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instruction, proposal_transaction_buffer_address)
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let (ix, proposal_transaction_buffer_address) = close_transaction_buffer(
        &program_id,
        &input.governance,
        &input.proposal,
        &input.token_owner_record,
        &input.governance_authority.pubkey(),
        &input.beneficiary.pubkey(),
        input.buffer_index,
    );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer,
            input.governance_authority,
            input.beneficiary,
        ]
        .into(),
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
        let beneficiary = Pubkey::new_unique();

        let (ix, _addr) = close_transaction_buffer(
            &SPL_GOVERNANCE_ID,
            &governance,
            &proposal,
            &token_owner_record,
            &governance_authority,
            &beneficiary,
            0u8,
        );

        assert_eq!(ix.program_id, SPL_GOVERNANCE_ID);
        assert!(!ix.data.is_empty());
        assert!(ix.accounts.len() >= 6);
    }
}
