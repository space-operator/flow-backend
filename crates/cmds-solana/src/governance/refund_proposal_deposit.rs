use std::str::FromStr;

use solana_sdk::instruction::AccountMeta;
use tracing::info;

use crate::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "refund_proposal_deposit";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/refund_proposal_deposit.json");
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
    pub proposal_deposit_payer: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn refund_proposal_deposit(
    program_id: &Pubkey,
    // Accounts
    proposal: &Pubkey,
    proposal_deposit_payer: &Pubkey,
    // Args
) -> (Instruction, Pubkey) {
    let seeds = [
        b"proposal-deposit",
        proposal.as_ref(),
        proposal_deposit_payer.as_ref(),
    ];
    let proposal_deposit_address = Pubkey::find_program_address(&seeds, program_id).0;
    info!("Proposal deposit address: {}", proposal_deposit_address);

    let accounts = vec![
        AccountMeta::new_readonly(*proposal, false),
        AccountMeta::new(proposal_deposit_address, false),
        AccountMeta::new(*proposal_deposit_payer, false),
    ];

    let data = GovernanceInstruction::RefundProposalDeposit {};

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instruction, proposal_deposit_address)
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();

    let (ix, proposal_deposit_address) =
        refund_proposal_deposit(&program_id, &input.proposal, &input.proposal_deposit_payer);

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
        instructions: [ix].into(),
    };

    let signature = ctx
        .execute(
            instructions,
            value::map!(
                "proposal_deposit_address" => proposal_deposit_address,
            ),
        )
        .await?
        .signature;

    Ok(Output { signature })
}
