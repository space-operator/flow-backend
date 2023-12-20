use std::str::FromStr;
use std::time::SystemTime;

use crate::{find_pda, prelude::*};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::AccountMeta;
use solana_program::{system_program, sysvar};
use solana_sdk::program_pack::Pack;
use solana_sdk::system_instruction;
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Mint;

const NAME: &str = "create_timelock";

const DEFINITION: &str = flow_lib::node_definition!("solana/streamflow/create_timelock.json");

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
    #[serde(with = "value::keypair")]
    fee_payer: Keypair,
    #[serde(with = "value::keypair")]
    sender: Keypair,
    #[serde(with = "value::pubkey")]
    recipient: Pubkey,
    #[serde(with = "value::keypair")]
    metadata: Keypair,
    #[serde(with = "value::pubkey")]
    mint_account: Pubkey,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct CreateStreamIx {
    ix: u8,
    metadata: StreamInstruction,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct StreamInstruction {
    start_time: u64,
    end_time: u64,
    deposited_amount: u64,
    total_amount: u64,
    period: u64,
    cliff: u64,
    cliff_amount: u64,
    cancelable_by_sender: bool,
    cancelable_by_recipient: bool,
    withdrawal_public: bool,
    transferable_by_sender: bool,
    transferable_by_recipient: bool,
    release_rate: u64,
    stream_name: String,
}

fn create_create_stream_instruction(
    sender: &Pubkey,
    sender_tokens: &Pubkey,
    recipient: &Pubkey,
    recipient_tokens: &Pubkey,
    metadata: &Pubkey,
    escrow_tokens: &Pubkey,
    mint: &Pubkey,
    timelock_program: &Pubkey,
    data: CreateStreamIx,
) -> Instruction {
    let accounts = [
        AccountMeta::new(*sender, true),
        AccountMeta::new(*sender_tokens, false),
        AccountMeta::new(*recipient, false),
        AccountMeta::new(*recipient_tokens, false),
        AccountMeta::new(*metadata, true),
        AccountMeta::new(*escrow_tokens, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
        AccountMeta::new_readonly(*timelock_program, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ]
    .to_vec();

    Instruction {
        program_id: *timelock_program,
        accounts,
        data: data.try_to_vec().unwrap(),
    }
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let timelock_program: Pubkey =
        Pubkey::from_str("8e72pYCDaxu3GqMfeQ5r8wFgoZSYk6oua1Qo9XpsZjX").unwrap();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let data: CreateStreamIx = CreateStreamIx {
        ix: 0,
        metadata: StreamInstruction {
            start_time: now + 5,
            end_time: now + 605,
            deposited_amount: spl_token::ui_amount_to_amount(20.0, 9),
            total_amount: spl_token::ui_amount_to_amount(20.0, 9),
            period: 0,
            cliff: 0,
            cliff_amount: 0,
            cancelable_by_sender: false,
            cancelable_by_recipient: false,
            withdrawal_public: false,
            transferable_by_sender: false,
            transferable_by_recipient: false,
            release_rate: 0,
            stream_name: "TheTestoooooooooor".to_string(),
        },
    };

    let escrow_tokens =
        Pubkey::find_program_address(&[input.metadata.pubkey().as_ref()], &timelock_program).0;

    let sender_tokens: Pubkey =
        get_associated_token_address(&input.sender.pubkey(), &input.mint_account);

    let recipient_tokens = get_associated_token_address(&input.recipient, &input.mint_account);

    let instruction = create_create_stream_instruction(
        &input.fee_payer.pubkey(),
        &sender_tokens,
        &input.recipient,
        &recipient_tokens,
        &input.metadata.pubkey(),
        &escrow_tokens,
        &input.mint_account,
        &timelock_program,
        data,
    );

    let ins = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.sender.clone_keypair(),
            input.metadata.clone_keypair(),
        ]
        .into(),
        instructions: vec![instruction].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

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
