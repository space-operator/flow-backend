use solana_program::instruction::AccountMeta;
use solana_sdk_ids::system_program;
use tracing::info;

use super::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID, Vote, with_realm_config_accounts};

const NAME: &str = "cast_vote";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/cast_vote.jsonc");
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
    #[serde(with = "value::pubkey")]
    pub voter_token_owner_record: Pubkey,

    pub governance_authority: Wallet,
    #[serde(with = "value::pubkey")]
    pub vote_governing_token_mint: Pubkey,
    #[serde(default, with = "value::pubkey::opt")]
    pub voter_weight_record: Option<Pubkey>,
    #[serde(default, with = "value::pubkey::opt")]
    pub max_voter_weight_record: Option<Pubkey>,
    pub vote: Vote,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn cast_vote(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    proposal_owner_record: &Pubkey,
    voter_token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    vote_governing_token_mint: &Pubkey,
    payer: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    max_voter_weight_record: Option<Pubkey>,
    // Args
    vote: Vote,
) -> (Instruction, Pubkey) {
    let seeds = [
        b"governance",
        proposal.as_ref(),
        voter_token_owner_record.as_ref(),
    ];
    let vote_record_address = Pubkey::find_program_address(&seeds, program_id).0;

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*proposal_owner_record, false),
        AccountMeta::new(*voter_token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(vote_record_address, false),
        AccountMeta::new_readonly(*vote_governing_token_mint, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    with_realm_config_accounts(
        program_id,
        &mut accounts,
        realm,
        voter_weight_record,
        max_voter_weight_record,
    );
    info!("accounts: {:?}", accounts);

    let data = GovernanceInstruction::CastVote { vote };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instruction, vote_record_address)
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let (ix, vote_record_address) = cast_vote(
        &program_id,
        &input.realm,
        &input.governance,
        &input.proposal,
        &input.proposal_owner_record,
        &input.voter_token_owner_record,
        &input.governance_authority.pubkey(),
        &input.vote_governing_token_mint,
        &input.fee_payer.pubkey(),
        input.voter_weight_record,
        input.max_voter_weight_record,
        input.vote,
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
                "vote_record_address" => vote_record_address,
            ),
        )
        .await?
        .signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::super::{SPL_GOVERNANCE_ID, Vote, VoteChoice};
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
        let voter_token_owner_record = Pubkey::new_unique();
        let governance_authority = Pubkey::new_unique();
        let vote_governing_token_mint = Pubkey::new_unique();
        let payer = Pubkey::new_unique();

        let (ix, _addr) = cast_vote(
            &SPL_GOVERNANCE_ID,
            &realm,
            &governance,
            &proposal,
            &proposal_owner_record,
            &voter_token_owner_record,
            &governance_authority,
            &vote_governing_token_mint,
            &payer,
            None,
            None,
            Vote::Approve(vec![VoteChoice {
                rank: 0,
                weight_percentage: 100,
            }]),
        );

        assert_eq!(ix.program_id, SPL_GOVERNANCE_ID);
        assert!(!ix.data.is_empty());
        assert!(ix.accounts.len() >= 10);
    }
}
