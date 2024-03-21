use std::str::FromStr;

use solana_sdk::{instruction::AccountMeta, loader_instruction::finalize, system_program};

use crate::prelude::*;

use super::{with_realm_config_accounts, GovernanceInstruction, Vote, SPL_GOVERNANCE_ID};

const NAME: &str = "finalize_vote";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/finalize_vote.json");
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
    #[serde(with = "value::pubkey")]
    pub realm: Pubkey,
    #[serde(with = "value::pubkey")]
    pub governance: Pubkey,
    #[serde(with = "value::pubkey")]
    pub proposal: Pubkey,
    #[serde(with = "value::pubkey")]
    pub proposal_owner_record: Pubkey,
    #[serde(with = "value::pubkey")]
    pub governing_token_mint: Pubkey,
    #[serde(with = "value::keypair")]
    pub governance_authority: Keypair,
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

pub fn finalize_vote(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    proposal_owner_record: &Pubkey,
    governing_token_mint: &Pubkey,
    max_voter_weight_record: Option<Pubkey>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*proposal_owner_record, false),
        AccountMeta::new_readonly(*governing_token_mint, false),
    ];

    with_realm_config_accounts(
        program_id,
        &mut accounts,
        realm,
        None,
        max_voter_weight_record,
    );

    let instruction = GovernanceInstruction::FinalizeVote {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();

    let (ix) = finalize_vote(
        &program_id,
        &input.realm,
        &input.governance,
        &input.proposal,
        &input.proposal_owner_record,
        &input.vote_governing_token_mint,
        input.max_voter_weight_record,
    );

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone_keypair()].into(),
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
