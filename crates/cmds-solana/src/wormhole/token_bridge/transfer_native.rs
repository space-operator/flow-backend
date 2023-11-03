use crate::prelude::*;

use borsh::BorshSerialize;
use rand::Rng;
use solana_program::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;

use super::{
    eth::hex_to_address, get_sequence_number, SequenceTracker, TokenBridgeInstructions,
    TransferNativeData,
};

// Command Name
const NAME: &str = "transfer_native";

const DEFINITION: &str = include_str!(
    "../../../../../node-definitions/solana/wormhole/token_bridge/transfer_native.json"
);

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub payer: Keypair,
    #[serde(with = "value::keypair")]
    pub message: Keypair,
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
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let wormhole_core_program_id =
        crate::wormhole::wormhole_core_program_id(ctx.cfg.solana_client.cluster);

    let token_bridge_program_id =
        crate::wormhole::token_bridge_program_id(ctx.cfg.solana_client.cluster);

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
    let nonce = rand::thread_rng().gen();

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
            AccountMeta::new_readonly(wormhole_core_program_id, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: (TokenBridgeInstructions::TransferNative, wrapped_data).try_to_vec()?,
    };

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_bubblegum::accounts::CreateTree,
        >())
        .await?;

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [input.payer.clone_keypair(), input.message.clone_keypair()].into(),
        instructions: [
            spl_token::instruction::approve(
                &spl_token::id(),
                &input.from,
                &authority_signer,
                &input.payer.pubkey(),
                &[],
                input.amount,
            )
            .unwrap(),
            ix,
        ]
        .into(),
        minimum_balance_for_rent_exemption,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let sequence_data: SequenceTracker = get_sequence_number(&ctx, sequence).await;

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "custody_key" => custody_key,
                "custody_signer" => custody_signer,
                "sequence" => sequence_data.sequence.to_string(),
                "emitter" => emitter.to_string(),
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
