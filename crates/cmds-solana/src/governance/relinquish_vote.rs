use std::str::FromStr;

use solana_program::instruction::AccountMeta;
use tracing::info;

use crate::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "relinquish_vote";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/relinquish_vote.json");
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
    pub token_owner_record: Pubkey,
    #[serde(with = "value::pubkey")]
    pub vote_governing_token_mint: Pubkey,
    pub governance_authority: Option<Wallet>,
    #[serde(default, with = "value::pubkey::opt")]
    pub beneficiary: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

#[allow(clippy::too_many_arguments)]
pub fn relinquish_vote(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    vote_governing_token_mint: &Pubkey,
    governance_authority: Option<Pubkey>,
    beneficiary: Option<Pubkey>,
) -> (Instruction, Pubkey) {
    let seeds = [
        b"governance",
        proposal.as_ref(),
        token_owner_record.as_ref(),
    ];
    let vote_record_address = Pubkey::find_program_address(&seeds, program_id).0;

    info!(
        "Relinquish Vote: vote_record_address: {}",
        vote_record_address
    );

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*token_owner_record, false),
        AccountMeta::new(vote_record_address, false),
        AccountMeta::new_readonly(*vote_governing_token_mint, false),
    ];

    if let Some(governance_authority) = governance_authority {
        accounts.push(AccountMeta::new_readonly(governance_authority, true));
        accounts.push(AccountMeta::new(beneficiary.unwrap(), false));
    }

    let data = GovernanceInstruction::RelinquishVote {};

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };

    (instruction, vote_record_address)
}
async fn run(mut ctx: CommandContextX, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();

    let (ix, vote_record_address) = relinquish_vote(
        &program_id,
        &input.realm,
        &input.governance,
        &input.proposal,
        &input.token_owner_record,
        &input.vote_governing_token_mint,
        input.governance_authority.as_ref().map(|k| k.pubkey()),
        input.beneficiary,
    );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: std::iter::once(input.fee_payer)
            .chain(input.governance_authority)
            .collect(),
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
