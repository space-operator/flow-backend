use std::str::FromStr;

use crate::prelude::*;
use crate::utils::anchor_sighash;
use solana_program::instruction::AccountMeta;
use solana_program::sysvar;
use solana_sdk_ids::system_program;
use spl_associated_token_account_interface::address::get_associated_token_address;

use super::{CreateData, CreateDataInput, FEE_ORACLE_ADDRESS, STRM_TREASURY, WITHDRAWOR_ADDRESS};

const NAME: &str = "create_streamflow_timelock";

const DEFINITION: &str = flow_lib::node_definition!("streamflow/create.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    fee_payer: Wallet,
    sender: Wallet,
    #[serde(with = "value::pubkey")]
    recipient: Pubkey,
    metadata: Wallet,
    #[serde(with = "value::pubkey")]
    mint_account: Pubkey,
    data: CreateDataInput,
    #[serde(default, with = "value::pubkey::opt")]
    partner: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

fn create_create_stream_instruction(
    sender: &Pubkey,
    sender_tokens: &Pubkey,
    recipient: &Pubkey,
    recipient_tokens: &Pubkey,
    metadata: &Pubkey,
    escrow_tokens: &Pubkey,
    streamflow_treasury_tokens: &Pubkey,
    partner: &Pubkey,
    partner_tokens: &Pubkey,
    mint: &Pubkey,
    timelock_program: &Pubkey,
    data: CreateData,
) -> Instruction {
    let accounts = [
        AccountMeta::new(*sender, true),
        AccountMeta::new(*sender_tokens, false),
        AccountMeta::new(*recipient, false),
        AccountMeta::new(*metadata, true),
        AccountMeta::new(*escrow_tokens, false),
        AccountMeta::new(*recipient_tokens, false),
        AccountMeta::new(Pubkey::from_str(STRM_TREASURY).unwrap(), false),
        AccountMeta::new(*streamflow_treasury_tokens, false),
        AccountMeta::new(Pubkey::from_str(WITHDRAWOR_ADDRESS).unwrap(), false),
        AccountMeta::new(*partner, false),
        AccountMeta::new(*partner_tokens, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(Pubkey::from_str(FEE_ORACLE_ADDRESS).unwrap(), false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
        AccountMeta::new_readonly(*timelock_program, false),
        AccountMeta::new_readonly(spl_token_interface::ID, false),
        AccountMeta::new_readonly(spl_associated_token_account_interface::program::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ]
    .to_vec();

    let discriminator = anchor_sighash("create");

    Instruction {
        program_id: *timelock_program,
        accounts,
        data: borsh::to_vec(&(discriminator, data)).unwrap(),
    }
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let timelock_program = crate::streamflow::streamflow_program_id(ctx.solana_config().cluster);

    let data: CreateData = input.data.into();

    let escrow_tokens = Pubkey::find_program_address(
        &[b"strm", input.metadata.pubkey().as_ref()],
        &timelock_program,
    )
    .0;

    let sender_tokens: Pubkey =
        get_associated_token_address(&input.sender.pubkey(), &input.mint_account);

    let recipient_tokens = get_associated_token_address(&input.recipient, &input.mint_account);

    let streamflow_treasury_tokens = get_associated_token_address(
        &Pubkey::from_str(STRM_TREASURY).unwrap(),
        &input.mint_account,
    );

    let partner = match &input.partner {
        Some(partner) => *partner,
        None => Pubkey::from_str(STRM_TREASURY).unwrap(),
    };

    let partner_tokens = get_associated_token_address(&partner, &input.mint_account);

    let instruction = create_create_stream_instruction(
        &input.fee_payer.pubkey(),
        &sender_tokens,
        &input.recipient,
        &recipient_tokens,
        &input.metadata.pubkey(),
        &escrow_tokens,
        &streamflow_treasury_tokens,
        &partner,
        &partner_tokens,
        &input.mint_account,
        &timelock_program,
        data,
    );

    let metadata_pubkey = input.metadata.pubkey();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.sender, input.metadata].into(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "escrow_tokens" => escrow_tokens,
                "sender_tokens" => sender_tokens,
                "recipient_tokens" => recipient_tokens,
                "metadata" => metadata_pubkey,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
