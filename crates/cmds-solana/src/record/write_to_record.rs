use solana_program::instruction::AccountMeta;

use crate::prelude::*;

use super::{RecordInstruction, record_program_id};

const NAME: &str = "write_to_record";

const DEFINITION: &str = flow_lib::node_definition!("/record/write_to_record.json");

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    fee_payer: Wallet,
    authority: Wallet,
    seed: String,
    offset: u64,
    data: String,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let record_program_id = record_program_id(ctx.solana_config().cluster);
    let record_account =
        Pubkey::create_with_seed(&input.authority.pubkey(), &input.seed, &record_program_id)
            .unwrap();

    let data = RecordInstruction::Write {
        offset: input.offset,
        data: input.data.into(),
    };

    let write_to_record_instruction = Instruction {
        program_id: record_program_id,
        accounts: vec![
            AccountMeta::new(record_account, false),
            AccountMeta::new_readonly(input.authority.pubkey(), false),
        ],
        data: borsh::to_vec(&data).unwrap(),
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: [write_to_record_instruction].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
