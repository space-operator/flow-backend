use solana_program::{instruction::AccountMeta, sysvar};
use solana_sdk_ids::system_program;

use super::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID, get_proposal_transaction_buffer_address};

const NAME: &str = "extend_transaction_buffer";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        flow_lib::node_definition!("/governance/extend_transaction_buffer.jsonc");
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
    /// Must be the same keypair that created the buffer
    pub creator: Wallet,
    pub buffer_index: u8,
    /// Additional buffer content chunk to append
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
/// 1. `[]` Proposal account
/// 2. `[writable]` ProposalTransactionBuffer account (PDA)
/// 3. `[signer]` Creator (payer who created the buffer)
/// 4. `[]` System program
/// 5. `[]` Rent sysvar
pub fn extend_transaction_buffer(
    program_id: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    creator: &Pubkey,
    buffer_index: u8,
    buffer: Vec<u8>,
) -> (Instruction, Pubkey) {
    let proposal_transaction_buffer_address =
        get_proposal_transaction_buffer_address(program_id, proposal, creator, &[buffer_index]);

    let accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new_readonly(*proposal, false),
        AccountMeta::new(proposal_transaction_buffer_address, false),
        AccountMeta::new_readonly(*creator, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    let data = GovernanceInstruction::ExtendTransactionBuffer {
        buffer_index,
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

    let (ix, proposal_transaction_buffer_address) = extend_transaction_buffer(
        &program_id,
        &input.governance,
        &input.proposal,
        &input.creator.pubkey(),
        input.buffer_index,
        input.buffer,
    );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.creator].into(),
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
        let creator = Pubkey::new_unique();

        let (ix, _addr) = extend_transaction_buffer(
            &SPL_GOVERNANCE_ID,
            &governance,
            &proposal,
            &creator,
            0u8,
            vec![1, 2, 3],
        );

        assert_eq!(ix.program_id, SPL_GOVERNANCE_ID);
        assert!(!ix.data.is_empty());
        assert!(ix.accounts.len() >= 6);
    }
}
