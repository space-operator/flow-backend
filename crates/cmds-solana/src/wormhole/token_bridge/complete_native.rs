use crate::wormhole::{PostVAAData, VAA};

use crate::prelude::*;

use borsh::BorshSerialize;
use solana_program::{instruction::AccountMeta, system_program, sysvar};
use solana_sdk::pubkey::Pubkey;
use wormhole_sdk::token::Message;

use super::{CompleteNativeData, PayloadTransfer, TokenBridgeInstructions};

// Command Name
const NAME: &str = "complete_native";

const DEFINITION: &str = flow_lib::node_definition!("wormhole/token_bridge/complete_native.json");

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
    #[serde(default, with = "value::pubkey::opt")]
    pub fee_recipient: Option<Pubkey>,
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

    let payload: PayloadTransfer = match input.payload {
        Message::Transfer {
            amount,
            token_address,
            token_chain,
            recipient,
            recipient_chain,
            fee,
        } => PayloadTransfer {
            amount,
            token_address: token_address.0,
            token_chain: token_chain.into(),
            to: recipient.into(),
            to_chain: recipient_chain.into(),
            fee,
        },
        // ignore other arms
        _ => {
            return Err(anyhow::anyhow!("Payload content not supported"));
        }
    };

    let to: Pubkey = Pubkey::from(payload.to.0);
    let mint = Pubkey::from(payload.token_address);

    let custody_key = Pubkey::find_program_address(&[mint.as_ref()], &token_bridge_program_id).0;
    let custody_signer =
        Pubkey::find_program_address(&[b"custody_signer"], &token_bridge_program_id).0;

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

    let ix = solana_program::instruction::Instruction {
        program_id: token_bridge_program_id,
        accounts: vec![
            AccountMeta::new(input.payer.pubkey(), true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(message, false),
            AccountMeta::new(claim_key, false),
            AccountMeta::new_readonly(endpoint, false),
            AccountMeta::new(to, false),
            if let Some(fee_r) = input.fee_recipient {
                AccountMeta::new(fee_r, false)
            } else {
                AccountMeta::new(to, false)
            },
            AccountMeta::new(custody_key, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(custody_signer, false),
            // Dependencies
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            // Program
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: (
            TokenBridgeInstructions::CompleteNative,
            CompleteNativeData {},
        )
            .try_to_vec()?,
    };

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [input.payer].into(),
        instructions: [ix].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "mint" => mint,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
