use solana_program::instruction::AccountMeta;
use solana_sdk_ids::system_program;

use super::prelude::*;

use super::{
    GovernanceConfig, GovernanceInstruction, SPL_GOVERNANCE_ID, with_realm_config_accounts,
};

const NAME: &str = "create_governance";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/create_governance.jsonc");
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
    pub governance_seed: Pubkey,
    #[serde(with = "value::pubkey")]
    pub token_owner_record: Pubkey,

    pub create_authority: Wallet,
    #[serde(default, with = "value::pubkey::opt")]
    pub voter_weight_record: Option<Pubkey>,
    pub config: GovernanceConfig,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn create_governance(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governance_seed: &Pubkey,
    token_owner_record: &Pubkey,
    payer: &Pubkey,
    create_authority: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    // Args
    config: GovernanceConfig,
) -> (Instruction, Pubkey) {
    let seeds = [
        b"account-governance",
        realm.as_ref(),
        governance_seed.as_ref(),
    ];
    let governance_address = Pubkey::find_program_address(&seeds, program_id).0;

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(governance_address, false),
        AccountMeta::new_readonly(*governance_seed, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*create_authority, true),
    ];

    with_realm_config_accounts(program_id, &mut accounts, realm, voter_weight_record, None);

    let data = GovernanceInstruction::CreateGovernance { config };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instruction, governance_address)
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let (ix, governance_address) = create_governance(
        &program_id,
        &input.realm,
        &input.governance_seed,
        &input.token_owner_record,
        &input.fee_payer.pubkey(),
        &input.create_authority.pubkey(),
        input.voter_weight_record,
        input.config,
    );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.create_authority].into(),
        instructions: [ix].into(),
    };

    let signature = ctx
        .execute(
            instructions,
            value::map!(
                "governance_address" => governance_address,
            ),
        )
        .await?
        .signature;

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
        let realm = Pubkey::new_unique();
        let governance_seed = Pubkey::new_unique();
        let token_owner_record = Pubkey::new_unique();
        let payer = Pubkey::new_unique();
        let create_authority = Pubkey::new_unique();

        let (ix, _addr) = create_governance(
            &SPL_GOVERNANCE_ID,
            &realm,
            &governance_seed,
            &token_owner_record,
            &payer,
            &create_authority,
            None,
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
        assert!(ix.accounts.len() >= 7);
    }
}
