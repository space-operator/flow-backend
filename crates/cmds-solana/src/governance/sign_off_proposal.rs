use std::str::FromStr;

use solana_sdk::instruction::AccountMeta;
use tracing::info;

use crate::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "sign_off_proposal";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/sign_off_proposal.json");
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
    #[serde(with = "value::keypair")]
    pub signatory: Keypair,
    #[serde(default, with = "value::pubkey::opt")]
    pub proposal_owner_record: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn sign_off_proposal(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    signatory: &Pubkey,
    proposal_owner_record: Option<&Pubkey>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*signatory, true),
    ];

    if let Some(proposal_owner_record) = proposal_owner_record {
        accounts.push(AccountMeta::new_readonly(*proposal_owner_record, false))
    } else {
        let seeds = [b"governance", proposal.as_ref(), signatory.as_ref()];
        let signatory_record_address = Pubkey::find_program_address(&seeds, program_id).0;
        info!("signatory_record_address: {}", signatory_record_address);
        accounts.push(AccountMeta::new(signatory_record_address, false));
    }

    let data = GovernanceInstruction::SignOffProposal;

    

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    }
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();

    let ix = sign_off_proposal(
        &program_id,
        &input.realm,
        &input.governance,
        &input.proposal,
        &input.signatory.pubkey(),
        input.proposal_owner_record.as_ref(),
    );

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone_keypair(), input.signatory.clone_keypair()].into(),
        instructions: [ix].into(),
    };

    let signature = ctx.execute(instructions, <_>::default()).await?.signature;

    Ok(Output { signature })
}
