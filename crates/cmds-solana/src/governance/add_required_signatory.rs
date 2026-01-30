use std::str::FromStr;

use solana_program::instruction::AccountMeta;
use solana_sdk_ids::system_program;
use tracing::info;

use crate::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "add_required_signatory";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/add_required_signatory.json");
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
    #[serde(with = "value::pubkey")]
    pub signatory: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn add_required_signatory(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    payer: &Pubkey,
    // Args
    signatory: &Pubkey,
) -> (Instruction, Pubkey) {
    let seeds = [
        b"required-signatory".as_ref(),
        governance.as_ref(),
        signatory.as_ref(),
    ];
    let required_signatory_address = Pubkey::find_program_address(&seeds, program_id).0;
    info!("required_signatory_address: {}", required_signatory_address);

    let accounts = vec![
        AccountMeta::new(*governance, true),
        AccountMeta::new(required_signatory_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let data = GovernanceInstruction::AddRequiredSignatory {
        signatory: *signatory,
    };

    let instruction = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instruction, required_signatory_address)
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = SPL_GOVERNANCE_ID;

    let (ix, required_signatory_address) = add_required_signatory(
        &program_id,
        &input.governance.pubkey(),
        &input.fee_payer.pubkey(),
        &input.signatory,
    );

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.governance].into(),
        instructions: [ix].into(),
    };

    let signature = ctx
        .execute(
            instructions,
            value::map!(
                "required_signatory_address" => required_signatory_address,
            ),
        )
        .await?
        .signature;

    Ok(Output { signature })
}
