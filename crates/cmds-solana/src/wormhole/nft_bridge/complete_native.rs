use crate::wormhole::{PostVAAData, VAA};

use crate::prelude::*;

use borsh::BorshSerialize;
use solana_program::{instruction::AccountMeta, system_program, sysvar};
use solana_sdk::pubkey::Pubkey;
use wormhole_sdk::nft::Message;

use super::{CompleteNativeData, NFTBridgeInstructions, PayloadTransfer};

// Command Name
const NAME: &str = "nft_complete_native";

const DEFINITION: &str = include_str!(
    "../../../../../node-definitions/solana/wormhole/nft_bridge/nft_complete_native.json"
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
    #[serde(with = "value::pubkey")]
    pub to_authority: Pubkey,
    pub vaa: bytes::Bytes,
    pub payload: wormhole_sdk::nft::Message,
    pub vaa_hash: bytes::Bytes,
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

    let vaa =
        VAA::deserialize(&input.vaa).map_err(|_| anyhow::anyhow!("Failed to deserialize VAA"))?;
    let vaa: PostVAAData = vaa.into();

    let message =
        Pubkey::find_program_address(&[b"PostedVAA", &input.vaa_hash], &wormhole_core_program_id).0;

    let claim_key = Pubkey::find_program_address(
        &[
            vaa.emitter_address.as_ref(),
            vaa.emitter_chain.to_be_bytes().as_ref(),
            vaa.sequence.to_be_bytes().as_ref(),
        ],
        &nft_bridge_program_id,
    )
    .0;

    let endpoint = Pubkey::find_program_address(
        &[
            vaa.emitter_chain.to_be_bytes().as_ref(),
            vaa.emitter_address.as_ref(),
        ],
        &nft_bridge_program_id,
    )
    .0;

    let payload: PayloadTransfer = match input.payload {
        Message::Transfer {
            nft_address,
            nft_chain,
            symbol,
            name,
            token_id,
            uri,
            to,
            to_chain,
        } => PayloadTransfer {
            token_address: nft_address.0,
            token_chain: nft_chain.into(),
            to: to.into(),
            to_chain: to_chain.into(),
            symbol: symbol.to_string(),
            name: name.to_string(),
            token_id: primitive_types::U256::from(token_id.0),
            uri: uri.to_string(),
        },
    };
    // https://github.com/wormhole-foundation/wormhole/blob/faa397ca4f5cca067a7cfff375ab193463aabe39/sdk/js/src/solana/nftBridge/program.ts#L37
    let mut mint = vec![0u8; 32];
    payload.token_id.to_big_endian(&mut mint);

    let mint = Pubkey::try_from(mint).map_err(|_| anyhow::anyhow!("Invalid mint"))?;

    let custody_key = Pubkey::find_program_address(&[mint.as_ref()], &nft_bridge_program_id).0;
    let custody_signer =
        Pubkey::find_program_address(&[b"custody_signer"], &nft_bridge_program_id).0;

    let associated_token =
        spl_associated_token_account::get_associated_token_address(&input.to_authority, &mint);

    let ix = solana_program::instruction::Instruction {
        program_id: nft_bridge_program_id,
        accounts: vec![
            AccountMeta::new(input.payer.pubkey(), true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(message, false),
            AccountMeta::new(claim_key, false),
            AccountMeta::new_readonly(endpoint, false),
            AccountMeta::new(associated_token, false),
            AccountMeta::new_readonly(input.to_authority, false),
            AccountMeta::new(custody_key, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(custody_signer, false),
            // Dependencies
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            // Program
            AccountMeta::new_readonly(wormhole_core_program_id, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
        data: (NFTBridgeInstructions::CompleteNative, CompleteNativeData {}).try_to_vec()?,
    };

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_bubblegum::accounts::CreateTree,
        >())
        .await?;

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [input.payer.clone_keypair()].into(),
        instructions: [ix].into(),
        minimum_balance_for_rent_exemption,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "token" => associated_token,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
