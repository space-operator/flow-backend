use crate::wormhole::token_bridge::{eth::hex_to_address, get_sequence_number_from_message};

use crate::prelude::*;

use borsh::BorshSerialize;

use rand::Rng;
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;

use super::{NFTBridgeInstructions, TransferWrappedData};

// Command Name
const NAME: &str = "nft_transfer_wrapped";

const DEFINITION: &str =
    flow_lib::node_definition!("wormhole/nft_bridge/nft_transfer_wrapped.json");

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
    #[serde(with = "value::pubkey")]
    pub mint: Pubkey,
    pub target_address: String,
    pub target_chain: u16,
    pub message: Wallet,
    pub from_owner: Wallet,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
    sequence: String,
}

async fn run(mut ctx: CommandContextX, input: Input) -> Result<Output, CommandError> {
    let wormhole_core_program_id =
        crate::wormhole::wormhole_core_program_id(ctx.solana_config().cluster);

    let nft_bridge_program_id = crate::wormhole::nft_bridge_program_id(ctx.solana_config().cluster);

    let config_key = Pubkey::find_program_address(&[b"config"], &nft_bridge_program_id).0;

    let wrapped_meta_key =
        Pubkey::find_program_address(&[b"meta", input.mint.as_ref()], &nft_bridge_program_id).0;

    // SPL Metadata
    let spl_metadata = Pubkey::find_program_address(
        &[
            b"metadata".as_ref(),
            mpl_token_metadata::ID.as_ref(),
            input.mint.as_ref(),
        ],
        &mpl_token_metadata::ID,
    )
    .0;

    let authority_signer =
        Pubkey::find_program_address(&[b"authority_signer"], &nft_bridge_program_id).0;

    let emitter = Pubkey::find_program_address(&[b"emitter"], &nft_bridge_program_id).0;

    let bridge_config = Pubkey::find_program_address(&[b"Bridge"], &wormhole_core_program_id).0;

    let sequence =
        Pubkey::find_program_address(&[b"Sequence", emitter.as_ref()], &wormhole_core_program_id).0;

    let fee_collector =
        Pubkey::find_program_address(&[b"fee_collector"], &wormhole_core_program_id).0;

    // TODO: use a real nonce
    let nonce = rand::thread_rng().r#gen();

    let wrapped_data = TransferWrappedData {
        nonce,
        target_address: hex_to_address(&input.target_address)?,
        target_chain: input.target_chain,
    };

    let from_ata = spl_associated_token_account::get_associated_token_address(
        &input.from_owner.pubkey(),
        &input.mint,
    );

    let ix = solana_program::instruction::Instruction {
        program_id: nft_bridge_program_id,
        accounts: vec![
            AccountMeta::new(input.payer.pubkey(), true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new(from_ata, false),
            AccountMeta::new_readonly(input.from_owner.pubkey(), true),
            AccountMeta::new(input.mint, false),
            AccountMeta::new_readonly(wrapped_meta_key, false),
            AccountMeta::new_readonly(spl_metadata, false),
            AccountMeta::new_readonly(authority_signer, false),
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
        data: (NFTBridgeInstructions::TransferWrapped, wrapped_data).try_to_vec()?,
    };

    let approve_ix = spl_token::instruction::approve(
        &spl_token::id(),
        &from_ata,
        &authority_signer,
        &input.from_owner.pubkey(),
        &[],
        1,
    )?;

    let message_pubkey = input.message.pubkey();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.from_owner, input.message].into(),
        instructions: [approve_ix, ix].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "wrapped_meta_key" => wrapped_meta_key,
                "emitter" => emitter,
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
