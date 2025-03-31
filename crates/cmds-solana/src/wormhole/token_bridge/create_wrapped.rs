use super::{CreateWrappedData, PayloadAssetMeta, TokenBridgeInstructions};
use crate::prelude::*;
use crate::wormhole::{PostVAAData, VAA};
use borsh::BorshSerialize;
use solana_program::{instruction::AccountMeta, system_program, sysvar};
use solana_program::pubkey::Pubkey;
use tracing::info;
use wormhole_sdk::token::Message;

// Command Name
const NAME: &str = "create_wrapped";

const DEFINITION: &str = flow_lib::node_definition!("wormhole/token_bridge/create_wrapped.json");

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
    pub vaa: bytes::Bytes,
    pub payload: wormhole_sdk::token::Message,
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

    let token_bridge_program_id =
        crate::wormhole::token_bridge_program_id(ctx.cfg.solana_client.cluster);

    let config_key = Pubkey::find_program_address(&[b"config"], &token_bridge_program_id).0;

    let vaa =
        VAA::deserialize(&input.vaa).map_err(|_| anyhow::anyhow!("Failed to deserialize VAA"))?;
    let vaa: PostVAAData = vaa.into();

    let payload: PayloadAssetMeta = match input.payload {
        Message::AssetMeta {
            token_address,
            token_chain,
            decimals,
            symbol,
            name,
        } => PayloadAssetMeta {
            token_address: token_address.0,
            token_chain: token_chain.into(),
            decimals,
            symbol: symbol.to_string(),
            name: name.to_string(),
        },
        // ignore other arms
        _ => {
            return Err(anyhow::anyhow!("Payload content not supported"));
        }
    };

    info!("payload: {:?}", payload);

    let message =
        Pubkey::find_program_address(&[b"PostedVAA", &input.vaa_hash], &wormhole_core_program_id).0;

    let claim_key = Pubkey::find_program_address(
        &[
            vaa.emitter_address.as_ref(),
            vaa.emitter_chain.to_be_bytes().as_ref(),
            vaa.sequence.to_be_bytes().as_ref(),
        ],
        &token_bridge_program_id,
    )
    .0;

    let endpoint = Pubkey::find_program_address(
        &[
            vaa.emitter_chain.to_be_bytes().as_ref(),
            vaa.emitter_address.as_ref(),
        ],
        &token_bridge_program_id,
    )
    .0;

    let mint = Pubkey::find_program_address(
        &[
            b"wrapped",
            payload.token_chain.to_be_bytes().as_ref(),
            payload.token_address.as_ref(),
        ],
        &token_bridge_program_id,
    )
    .0;

    info!("payload token address: {:?}", payload.token_address);

    let mint_meta =
        Pubkey::find_program_address(&[b"meta", mint.as_ref()], &token_bridge_program_id).0;

    let mint_authority =
        Pubkey::find_program_address(&[b"mint_signer"], &token_bridge_program_id).0;

    // SPL Metadata
    let spl_metadata = Pubkey::find_program_address(
        &[
            b"metadata".as_ref(),
            mpl_token_metadata::ID.as_ref(),
            mint.as_ref(),
        ],
        &mpl_token_metadata::ID,
    )
    .0;

    let ix = solana_program::instruction::Instruction {
        program_id: token_bridge_program_id,
        accounts: vec![
            AccountMeta::new(input.payer.pubkey(), true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(endpoint, false),
            AccountMeta::new_readonly(message, false),
            AccountMeta::new(claim_key, false),
            AccountMeta::new(mint, false),
            AccountMeta::new(mint_meta, false),
            AccountMeta::new(spl_metadata, false),
            AccountMeta::new_readonly(mint_authority, false),
            // Dependencies
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            // Program
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(mpl_token_metadata::ID, false),
        ],
        data: (TokenBridgeInstructions::CreateWrapped, CreateWrappedData {}).try_to_vec()?,
    };

    info!("ix: {:?}", ix);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer].into(),
        instructions: [ix].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "spl_metadata" => spl_metadata,
                "mint_metadata" => mint_meta,
                "mint" => mint,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
