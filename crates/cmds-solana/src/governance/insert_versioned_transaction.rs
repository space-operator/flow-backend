use solana_program::instruction::AccountMeta;
use solana_sdk_ids::system_program;

use super::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID, get_proposal_versioned_transaction_address};

const NAME: &str = "insert_versioned_transaction";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        flow_lib::node_definition!("/governance/insert_versioned_transaction.jsonc");
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
    pub option_index: u8,
    pub ephemeral_signers: u8,
    pub transaction_index: u16,
    /// Serialized versioned transaction message
    pub transaction_message: Vec<u8>,
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
/// 4. `[writable]` ProposalVersionedTransaction account (PDA)
/// 5. `[writable, signer]` Payer
/// 6. `[]` System program
#[allow(clippy::too_many_arguments)]
pub fn insert_versioned_transaction(
    program_id: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    payer: &Pubkey,
    option_index: u8,
    ephemeral_signers: u8,
    transaction_index: u16,
    transaction_message: Vec<u8>,
) -> (Instruction, Pubkey) {
    let proposal_versioned_tx_address = get_proposal_versioned_transaction_address(
        program_id,
        proposal,
        &[option_index],
        &transaction_index.to_le_bytes(),
    );

    let accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(proposal_versioned_tx_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let data = GovernanceInstruction::InsertVersionedTransaction {
        option_index,
        ephemeral_signers,
        transaction_index,
        transaction_message,
    };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instruction, proposal_versioned_tx_address)
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let (ix, proposal_versioned_tx_address) = insert_versioned_transaction(
        &program_id,
        &input.governance,
        &input.proposal,
        &input.token_owner_record,
        &input.governance_authority.pubkey(),
        &input.fee_payer.pubkey(),
        input.option_index,
        input.ephemeral_signers,
        input.transaction_index,
        input.transaction_message,
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
                "proposal_versioned_transaction_address" => proposal_versioned_tx_address,
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

        let (ix, _addr) = insert_versioned_transaction(
            &SPL_GOVERNANCE_ID,
            &governance,
            &proposal,
            &token_owner_record,
            &governance_authority,
            &payer,
            0u8,
            0u8,
            0u16,
            vec![1, 2, 3],
        );

        assert_eq!(ix.program_id, SPL_GOVERNANCE_ID);
        assert!(!ix.data.is_empty());
        assert!(ix.accounts.len() >= 7);
    }
}
