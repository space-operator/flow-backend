use solana_program::instruction::AccountMeta;

use super::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "execute_versioned_transaction";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        flow_lib::node_definition!("/governance/execute_versioned_transaction.jsonc");
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
    pub proposal_versioned_transaction: Pubkey,
    /// Account metas required by the versioned transaction message.
    #[serde(default)]
    pub remaining_accounts: Vec<AccountMeta>,
    pub additional_signers: Option<Vec<Wallet>>,
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
/// 2. `[writable]` ProposalVersionedTransaction account
/// + remaining accounts from the versioned transaction message
pub fn execute_versioned_transaction(
    program_id: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    proposal_versioned_transaction: &Pubkey,
    remaining_accounts: &[AccountMeta],
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*proposal_versioned_transaction, false),
    ];

    accounts.extend_from_slice(remaining_accounts);

    let data = GovernanceInstruction::ExecuteVersionedTransaction;

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    }
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let ix = execute_versioned_transaction(
        &program_id,
        &input.governance,
        &input.proposal,
        &input.proposal_versioned_transaction,
        &input.remaining_accounts,
    );

    let signers = input.additional_signers.into_iter().flatten().collect();

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers,
        instructions: [ix].into(),
    };

    let signature = ctx.execute(instructions, <_>::default()).await?.signature;

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
        let proposal_versioned_transaction = Pubkey::new_unique();

        let ix = execute_versioned_transaction(
            &SPL_GOVERNANCE_ID,
            &governance,
            &proposal,
            &proposal_versioned_transaction,
            &[],
        );

        assert_eq!(ix.program_id, SPL_GOVERNANCE_ID);
        assert!(!ix.data.is_empty());
        assert!(ix.accounts.len() >= 3);
    }
}
