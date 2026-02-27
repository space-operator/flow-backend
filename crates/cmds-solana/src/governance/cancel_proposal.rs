use solana_program::instruction::AccountMeta;

use super::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "cancel_proposal";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/cancel_proposal.jsonc");
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
    pub realm: Pubkey,
    #[serde(with = "value::pubkey")]
    pub governance: Pubkey,
    #[serde(with = "value::pubkey")]
    pub proposal: Pubkey,
    #[serde(with = "value::pubkey")]
    pub proposal_owner_record: Pubkey,
    pub governance_authority: Wallet,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn cancel_proposal(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    proposal_owner_record: &Pubkey,
    governance_authority: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*proposal_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
    ];

    let instruction = GovernanceInstruction::CancelProposal {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let ix = cancel_proposal(
        &program_id,
        &input.realm,
        &input.governance,
        &input.proposal,
        &input.proposal_owner_record,
        &input.governance_authority.pubkey(),
    );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.governance_authority].into(),
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
        let realm = Pubkey::new_unique();
        let governance = Pubkey::new_unique();
        let proposal = Pubkey::new_unique();
        let proposal_owner_record = Pubkey::new_unique();
        let governance_authority = Pubkey::new_unique();

        let ix = cancel_proposal(
            &SPL_GOVERNANCE_ID,
            &realm,
            &governance,
            &proposal,
            &proposal_owner_record,
            &governance_authority,
        );

        assert_eq!(ix.program_id, SPL_GOVERNANCE_ID);
        assert!(!ix.data.is_empty());
        assert!(ix.accounts.len() >= 5);
    }
}
