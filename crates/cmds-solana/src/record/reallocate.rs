use crate::prelude::*;
use solana_program::rent::Rent;
use solana_system_interface::instruction;
use tracing::info;

// use super::RecordData;

use spl_record::instruction as record_instruction;

const NAME: &str = "reallocate";

const DEFINITION: &str = flow_lib::node_definition!("/record/reallocate.json");

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    fee_payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    account: Pubkey,
    authority: Wallet,
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
    let current_account_length = ctx
        .solana_client()
        .get_account(&input.account)
        .await?
        .data
        .len();

    let new_data_length = input.data.len() as u64;

    info!("current_account_length: {}", current_account_length);
    info!("new_data_length: {}", new_data_length);

    // let expected_account_data_length = RecordData::WRITABLE_START_INDEX
    //     .checked_add(new_data_length as usize)
    //     .unwrap();

    let delta_account_data_length = new_data_length.saturating_sub(current_account_length as u64);
    let additional_lamports_needed =
        Rent::default().minimum_balance(delta_account_data_length as usize);

    let instruction =
        record_instruction::reallocate(&input.account, &input.authority.pubkey(), new_data_length);

    let transfer_instruction = instruction::transfer(
        &input.fee_payer.pubkey(),
        &input.account,
        additional_lamports_needed,
    );

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: [transfer_instruction, instruction].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
