use solana_program::instruction::AccountMeta;

use super::prelude::*;

use super::{GovernanceConfig, GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "set_governance_config";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/set_governance_config.jsonc");
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

    pub governance: Wallet,
    pub config: GovernanceConfig,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn set_governance_config(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    // Args
    config: GovernanceConfig,
) -> Instruction {
    let accounts = vec![AccountMeta::new(*governance, true)];

    let instruction = GovernanceInstruction::SetGovernanceConfig { config };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let ix = set_governance_config(&program_id, &input.governance.pubkey(), input.config);

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.governance].into(),
        instructions: [ix].into(),
    };

    let signature = ctx.execute(instructions, <_>::default()).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::super::{GovernanceConfig, SPL_GOVERNANCE_ID, VoteThreshold, VoteTipping};
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_instruction_builder() {
        let governance = Pubkey::new_unique();

        let ix = set_governance_config(
            &SPL_GOVERNANCE_ID,
            &governance,
            GovernanceConfig {
                community_vote_threshold: VoteThreshold::YesVotePercentage(60),
                min_community_weight_to_create_proposal: 1,
                transactions_hold_up_time: 0,
                voting_base_time: 3600,
                community_vote_tipping: VoteTipping::Strict,
                council_vote_threshold: VoteThreshold::YesVotePercentage(60),
                council_veto_vote_threshold: VoteThreshold::Disabled,
                min_council_weight_to_create_proposal: 1,
                council_vote_tipping: VoteTipping::Strict,
                community_veto_vote_threshold: VoteThreshold::Disabled,
                voting_cool_off_time: 0,
                deposit_exempt_proposal_count: 10,
            },
        );

        assert_eq!(ix.program_id, SPL_GOVERNANCE_ID);
        assert!(!ix.data.is_empty());
        assert!(!ix.accounts.is_empty());
    }
}
