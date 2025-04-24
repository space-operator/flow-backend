use std::str::FromStr;

use solana_program::instruction::AccountMeta;

use crate::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "remove_transaction";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/remove_transaction.json");
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
    pub proposal_transaction: Pubkey,
    #[serde(with = "value::pubkey")]
    pub token_owner_record: Pubkey,

    pub governance_authority: Wallet,
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

pub fn remove_transaction(
    program_id: &Pubkey,
    // Accounts
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    proposal_transaction: &Pubkey,
    beneficiary: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(*proposal_transaction, false),
        AccountMeta::new(*beneficiary, false),
    ];

    let instruction = GovernanceInstruction::RemoveTransaction {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}
async fn run(mut ctx: CommandContextX, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();

    let ix = remove_transaction(
        &program_id,
        &input.proposal,
        &input.token_owner_record,
        &input.governance_authority.pubkey(),
        &input.proposal_transaction,
        &input.beneficiary,
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
