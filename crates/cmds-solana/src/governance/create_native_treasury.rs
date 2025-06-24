use std::str::FromStr;

use solana_program::instruction::AccountMeta;
use solana_sdk_ids::system_program;

use crate::prelude::*;

use super::{GovernanceInstruction, SPL_GOVERNANCE_ID};

const NAME: &str = "create_native_treasury";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("/governance/create_native_treasury.json");
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
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

pub fn create_native_treasury(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    payer: &Pubkey,
) -> (Instruction, Pubkey) {
    let seeds = [b"native-treasury", governance.as_ref()];
    let native_treasury_address = Pubkey::find_program_address(&seeds, program_id).0;

    let accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(native_treasury_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let data = GovernanceInstruction::CreateNativeTreasury {};

    let instructions = Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&data).unwrap(),
    };
    (instructions, native_treasury_address)
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let program_id = Pubkey::from_str(SPL_GOVERNANCE_ID).unwrap();

    let (ix, native_treasury_address) =
        create_native_treasury(&program_id, &input.governance, &input.fee_payer.pubkey());

    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
        instructions: [ix].into(),
    };

    let signature = ctx
        .execute(
            instructions,
            value::map!(
                "native_treasury_address" => native_treasury_address,
            ),
        )
        .await?
        .signature;

    Ok(Output { signature })
}
