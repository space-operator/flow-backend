use crate::wormhole::ForeignAddress;
use std::str::FromStr;

use crate::prelude::*;

use borsh::BorshSerialize;
use primitive_types::U256;
use rand::Rng;
use solana_program::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use wormhole_sdk::Address;

use super::{NFTBridgeInstructions, TransferWrappedData};

// Command Name
const NAME: &str = "nft_transfer_wrapped";

const DEFINITION: &str = include_str!(
    "../../../../../node-definitions/solana/wormhole/nft_bridge/nft_transfer_wrapped.json"
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
    pub token_chain: u16,
    pub token_address: ForeignAddress,
    pub token_id: String,
    pub amount: u64,
    pub fee: u64,
    pub target_address: Address,
    pub target_chain: u16,
    #[serde(with = "value::keypair")]
    pub message: Keypair,
    #[serde(with = "value::pubkey")]
    pub from: Pubkey,
    #[serde(with = "value::keypair")]
    pub from_owner: Keypair,
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

    let nft_bridge_program_id =
        crate::wormhole::nft_bridge_program_id(ctx.cfg.solana_client.cluster);

    let config_key = Pubkey::find_program_address(&[b"config"], &nft_bridge_program_id).0;

    // Convert token id
    let token_id_input =
        U256::from_str(&input.token_id).map_err(|_| anyhow::anyhow!("Invalid token id"))?;
    let mut token_id = vec![0u8; 32];
    token_id_input.to_big_endian(&mut token_id);

    let wrapped_mint_key = Pubkey::find_program_address(
        &[
            b"wrapped",
            input.token_chain.to_be_bytes().as_ref(),
            &input.token_address,
            &token_id,
        ],
        &nft_bridge_program_id,
    )
    .0;

    let wrapped_meta_key = Pubkey::find_program_address(
        &[b"meta", wrapped_mint_key.as_ref()],
        &nft_bridge_program_id,
    )
    .0;

    // SPL Metadata
    let spl_metadata = Pubkey::find_program_address(
        &[
            b"metadata".as_ref(),
            mpl_token_metadata::ID.as_ref(),
            wrapped_mint_key.as_ref(),
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
    let nonce = rand::thread_rng().gen();

    let wrapped_data = TransferWrappedData {
        nonce,
        target_address: super::Address(input.target_address.0),
        target_chain: input.target_chain,
    };

    let ix = solana_program::instruction::Instruction {
        program_id: nft_bridge_program_id,
        accounts: vec![
            AccountMeta::new(input.payer.pubkey(), true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new(input.from, false),
            AccountMeta::new_readonly(input.from_owner.pubkey(), true),
            AccountMeta::new(wrapped_mint_key, false),
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

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_bubblegum::accounts::CreateTree,
        >())
        .await?;

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer.clone_keypair(),
            input.from_owner.clone_keypair(),
            input.message.clone_keypair(),
        ]
        .into(),
        instructions: [ix].into(),
        minimum_balance_for_rent_exemption,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "wrapped_mint_key" => wrapped_mint_key,
                "wrapped_meta_key" => wrapped_meta_key,
                "spl_metadata" => spl_metadata,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
