use crate::prelude::*;

use borsh::BorshSerialize;
use rand::Rng;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;

use super::{
    TokenBridgeInstructions, TransferNativeData, eth::hex_to_address,
    get_sequence_number_from_message,
};

// Command Name
const NAME: &str = "transfer_native";

const DEFINITION: &str = flow_lib::node_definition!("wormhole/token_bridge/transfer_native.json");

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
    pub payer: Wallet,
    pub message: Wallet,
    #[serde(with = "value::pubkey")]
    pub from: Pubkey,
    #[serde(with = "value::pubkey")]
    pub mint: Pubkey,
    // 1 = 1,000,000,000
    pub amount: u64,
    pub fee: u64,
    pub target_address: String,
    pub target_chain: u16,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
    sequence: String,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let wormhole_core_program_id =
        crate::wormhole::wormhole_core_program_id(ctx.solana_config().cluster);

    let token_bridge_program_id =
        crate::wormhole::token_bridge_program_id(ctx.solana_config().cluster);

    let config_key = Pubkey::find_program_address(&[b"config"], &token_bridge_program_id).0;

    let mint = input.mint;

    let custody_key = Pubkey::find_program_address(&[mint.as_ref()], &token_bridge_program_id).0;

    let authority_signer =
        Pubkey::find_program_address(&[b"authority_signer"], &token_bridge_program_id).0;

    let custody_signer =
        Pubkey::find_program_address(&[b"custody_signer"], &token_bridge_program_id).0;

    let emitter = Pubkey::find_program_address(&[b"emitter"], &token_bridge_program_id).0;

    let bridge_config = Pubkey::find_program_address(&[b"Bridge"], &wormhole_core_program_id).0;

    let sequence =
        Pubkey::find_program_address(&[b"Sequence", emitter.as_ref()], &wormhole_core_program_id).0;

    let fee_collector =
        Pubkey::find_program_address(&[b"fee_collector"], &wormhole_core_program_id).0;

    // TODO: use a real nonce
    let nonce = rand::thread_rng().r#gen();

    let address = hex_to_address(&input.target_address).map_err(anyhow::Error::msg)?;

    let wrapped_data = TransferNativeData {
        nonce,
        amount: input.amount,
        fee: input.fee,
        target_address: address,
        target_chain: input.target_chain,
    };

    let ix = solana_program::instruction::Instruction {
        program_id: token_bridge_program_id,
        accounts: vec![
            AccountMeta::new(input.payer.pubkey(), true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new(input.from, false),
            AccountMeta::new(mint, false),
            AccountMeta::new(custody_key, false),
            AccountMeta::new_readonly(authority_signer, false),
            AccountMeta::new_readonly(custody_signer, false),
            AccountMeta::new(bridge_config, false),
            AccountMeta::new(input.message.pubkey(), true),
            AccountMeta::new_readonly(emitter, false),
            AccountMeta::new(sequence, false),
            AccountMeta::new(fee_collector, false),
            AccountMeta::new_readonly(solana_program::sysvar::clock::id(), false),
            // Dependencies
            AccountMeta::new_readonly(solana_program::sysvar::rent::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            // Program
            AccountMeta::new_readonly(spl_token_interface::ID, false),
            AccountMeta::new_readonly(wormhole_core_program_id, false),
        ],
        data: (TokenBridgeInstructions::TransferNative, wrapped_data).try_to_vec()?,
    };

    let instructions = [
        spl_token::instruction::approve(
            &spl_token_interface::ID,
            &input.from,
            &authority_signer,
            &input.payer.pubkey(),
            &[],
            input.amount,
        )
        .unwrap(),
        ix,
    ]
    .into();

    let message_pubkey = input.message.pubkey();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.message].into(),
        instructions,
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

    // let sequence_data: SequenceTracker = get_sequence_number(&ctx, sequence).await?;

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "custody" => custody_key,
                "custody_signer" => custody_signer,
                // "sequence" => sequence_data.sequence.to_string(),
                "emitter" => emitter.to_string(),
            },
        )
        .await?
        .signature;
    let sequence = get_sequence_number_from_message(&ctx, message_pubkey).await?;

    Ok(Output {
        signature,
        sequence,
    })
}
