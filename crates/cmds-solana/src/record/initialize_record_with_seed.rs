use std::mem::size_of;

use anchor_spl::token_2022::spl_token_2022::pod::pod_get_packed_len;
use solana_sdk::{instruction::AccountMeta, rent::Rent, system_instruction};

use crate::prelude::*;

use super::{record_program_id, RecordData, RecordInstruction};

const NAME: &str = "initialize_record_with_seed";

const DEFINITION: &str = flow_lib::node_definition!("/record/initialize_record_with_seed.json");

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
    #[serde(with = "value::keypair")]
    fee_payer: Keypair,
    #[serde(with = "value::keypair")]
    authority: Keypair,
    seed: String,
    data: String,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
    #[serde(with = "value::pubkey")]
    account: Pubkey,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let record_program_id = record_program_id(ctx.cfg.solana_client.cluster);

    let account =
        Pubkey::create_with_seed(&input.authority.pubkey(), &input.seed, &record_program_id)
            .unwrap();

    let data = input.data.as_bytes();

    let account_length = pod_get_packed_len::<RecordData>()
        .checked_add(data.len())
        .unwrap();

    let create_account_instruction = system_instruction::create_account_with_seed(
        &input.fee_payer.pubkey(),
        &account,
        &input.authority.pubkey(),
        &input.seed,
        1.max(Rent::default().minimum_balance(account_length)),
        account_length as u64,
        &record_program_id,
    );

    let initialize_record_instruction = Instruction {
        program_id: record_program_id,
        accounts: vec![
            AccountMeta::new(account, false),
            AccountMeta::new_readonly(input.authority.pubkey(), false),
        ],
        data: borsh::to_vec(&RecordInstruction::Initialize).unwrap(),
    };

    let data = RecordInstruction::Write {
        offset: 0,
        data: data.to_vec(),
    };

    let write_to_record_instruction = Instruction {
        program_id: record_program_id,
        accounts: vec![
            AccountMeta::new(account, false),
            AccountMeta::new_readonly(input.authority.pubkey(), false),
        ],
        data: borsh::to_vec(&data).unwrap(),
    };

    let ins = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.authority.clone_keypair(),
        ]
        .into(),
        instructions: [
            create_account_instruction,
            initialize_record_instruction,
            write_to_record_instruction,
        ]
        .into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(ins, value::map! { "account" => account })
        .await?
        .signature;

    Ok(Output { signature, account })
}
