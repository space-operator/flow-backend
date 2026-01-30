use solana_program::{instruction::AccountMeta, sysvar};
use solana_sdk_ids::system_program;

use crate::prelude::*;

use super::{GovernanceInstruction, InstructionData, SPL_GOVERNANCE_ID};

const NAME: &str = "insert_transaction";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/insert_transaction.json");
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
    pub token_owner_record: Pubkey,

    pub governance_authority: Wallet,
    pub option_index: u8,
    pub index: u16,
    pub instructions: Vec<Instruction>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

#[allow(clippy::too_many_arguments)]
pub fn insert_transaction(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    payer: &Pubkey,
    // Args
    option_index: u8,
    index: u16,
    instructions: Vec<InstructionData>,
) -> (Instruction, Pubkey) {
    let seeds = [
        b"governance",
        proposal.as_ref(),
        &option_index.to_le_bytes(),
        &index.to_le_bytes(),
    ];
    let proposal_transaction_address = Pubkey::find_program_address(&seeds, program_id).0;

    let accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(proposal_transaction_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    let data = GovernanceInstruction::InsertTransaction {
        option_index,
        index,
        legacy: 0,
        instructions,
    };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instruction, proposal_transaction_address)
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let (ix, proposal_transaction_address) = insert_transaction(
        &program_id,
        &input.governance,
        &input.proposal,
        &input.token_owner_record,
        &input.governance_authority.pubkey(),
        &input.fee_payer.pubkey(),
        input.option_index,
        input.index,
        input
            .instructions
            .into_iter()
            .map(|i| i.into())
            .collect::<Vec<InstructionData>>(),
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
                "proposal_transaction_address" => proposal_transaction_address,),
        )
        .await?
        .signature;

    Ok(Output { signature })
}
