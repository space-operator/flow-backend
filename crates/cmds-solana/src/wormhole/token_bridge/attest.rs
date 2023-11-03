use crate::prelude::*;

use borsh::BorshSerialize;
use rand::Rng;
use solana_program::{instruction::AccountMeta, system_program, sysvar};
use solana_sdk::pubkey::Pubkey;

use super::{get_sequence_number, AttestTokenData, SequenceTracker, TokenBridgeInstructions};

// Command Name
const NAME: &str = "attest_token";

const DEFINITION: &str =
    include_str!("../../../../../node-definitions/solana/wormhole/token_bridge/attest.json");

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
    pub mint: Pubkey,
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

    // TODO: use a real nonce
    let nonce = rand::thread_rng().gen();

    let config_key = Pubkey::find_program_address(&[b"config"], &token_bridge_program_id).0;

    let emitter = Pubkey::find_program_address(&[b"emitter"], &token_bridge_program_id).0;

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

    // Mint Metadata
    let seeds = &[b"meta".as_ref(), input.mint.as_ref()];
    let mint_meta = Pubkey::find_program_address(seeds, &token_bridge_program_id).0;

    let bridge = Pubkey::find_program_address(&[b"Bridge"], &wormhole_core_program_id).0;

    let fee_collector =
        Pubkey::find_program_address(&[b"fee_collector"], &wormhole_core_program_id).0;

    let sequence =
        Pubkey::find_program_address(&[b"Sequence", emitter.as_ref()], &wormhole_core_program_id).0;

    let ix = solana_program::instruction::Instruction {
        program_id: token_bridge_program_id,
        accounts: vec![
            AccountMeta::new(input.payer.pubkey(), true),
            AccountMeta::new(config_key, false),
            AccountMeta::new_readonly(input.mint, false),
            AccountMeta::new_readonly(mint_meta, false),
            AccountMeta::new_readonly(spl_metadata, false),
            // Bridge accounts
            AccountMeta::new(bridge, false),
            AccountMeta::new(input.message.pubkey(), true),
            AccountMeta::new_readonly(emitter, false),
            AccountMeta::new(sequence, false),
            AccountMeta::new(fee_collector, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            // Dependencies
            AccountMeta::new(sysvar::rent::id(), false),
            AccountMeta::new(system_program::id(), false),
            // Program
            AccountMeta::new_readonly(wormhole_core_program_id, false),
        ],
        data: (
            TokenBridgeInstructions::AttestToken,
            AttestTokenData { nonce },
        )
            .try_to_vec()?,
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
        instructions: [ix].into(),
        minimum_balance_for_rent_exemption,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let sequence_data: SequenceTracker = get_sequence_number(&ctx, sequence).await;

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "SPL_metadata" => spl_metadata,
                "mint_metadata" => mint_meta,
                "emitter"=>emitter.to_string(),
                "sequence"=>sequence_data.sequence.to_string(),
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
