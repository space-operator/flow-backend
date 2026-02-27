use solana_program::instruction::AccountMeta;

use super::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "remove_versioned_transaction";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        flow_lib::node_definition!("/governance/remove_versioned_transaction.jsonc");
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
    pub proposal: Pubkey,
    #[serde(with = "value::pubkey")]
    pub token_owner_record: Pubkey,
    pub governance_authority: Wallet,
    #[serde(with = "value::pubkey")]
    pub proposal_versioned_transaction: Pubkey,
    #[serde(with = "value::pubkey")]
    pub beneficiary: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

/// v3.1.2 account layout:
/// 0. `[writable]` Proposal account
/// 1. `[]` TokenOwnerRecord account of the Proposal owner
/// 2. `[signer]` Governance Authority (Token Owner or Governance Delegate)
/// 3. `[writable]` ProposalVersionedTransaction account
/// 4. `[writable]` Beneficiary account (receives rent)
pub fn remove_versioned_transaction(
    program_id: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    proposal_versioned_transaction: &Pubkey,
    beneficiary: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(*proposal_versioned_transaction, false),
        AccountMeta::new(*beneficiary, false),
    ];

    let data = GovernanceInstruction::RemoveVersionedTransaction;

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    }
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let ix = remove_versioned_transaction(
        &program_id,
        &input.proposal,
        &input.token_owner_record,
        &input.governance_authority.pubkey(),
        &input.proposal_versioned_transaction,
        &input.beneficiary,
    );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.governance_authority].into(),
        instructions: [ix].into(),
    };

    let signature = ctx.execute(instructions, value::map!()).await?.signature;

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
        let proposal = Pubkey::new_unique();
        let token_owner_record = Pubkey::new_unique();
        let governance_authority = Pubkey::new_unique();
        let proposal_versioned_transaction = Pubkey::new_unique();
        let beneficiary = Pubkey::new_unique();

        let ix = remove_versioned_transaction(
            &SPL_GOVERNANCE_ID,
            &proposal,
            &token_owner_record,
            &governance_authority,
            &proposal_versioned_transaction,
            &beneficiary,
        );

        assert_eq!(ix.program_id, SPL_GOVERNANCE_ID);
        assert!(!ix.data.is_empty());
        assert!(ix.accounts.len() >= 5);
    }
}
