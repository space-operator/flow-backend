use super::{Address, CompleteWrappedData, PayloadTransfer, TokenBridgeInstructions};
use crate::prelude::*;
use crate::wormhole::{PostVAAData, VAA};
use solana_commitment_config::CommitmentConfig;
use solana_program::pubkey::Pubkey;
use solana_program::{instruction::AccountMeta, sysvar};
use solana_sdk_ids::system_program;
use tracing::info;
use wormhole_sdk::token::Message;

// Command Name
const NAME: &str = "complete_transfer_wrapped";

const DEFINITION: &str =
    flow_lib::node_definition!("wormhole/token_bridge/complete_transfer_wrapped.json");

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

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let wormhole_core_program_id =
        crate::wormhole::wormhole_core_program_id(ctx.solana_config().cluster);

    let token_bridge_program_id =
        crate::wormhole::token_bridge_program_id(ctx.solana_config().cluster);

    let config_key = Pubkey::find_program_address(&[b"config"], &token_bridge_program_id).0;

    let vaa =
        VAA::deserialize(&input.vaa).map_err(|_| anyhow::anyhow!("Failed to deserialize VAA"))?;
    let vaa: PostVAAData = vaa.into();

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
            to: Address(recipient.0),
            to_chain: recipient_chain.into(),
            fee,
        },
        // ignore other arms
        _ => {
            return Err(anyhow::anyhow!("Payload content not supported"));
        }
    };

    let to = Pubkey::from(payload.to.0);

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

    let mint_meta =
        Pubkey::find_program_address(&[b"meta", mint.as_ref()], &token_bridge_program_id).0;

    let mint_authority =
        Pubkey::find_program_address(&[b"mint_signer"], &token_bridge_program_id).0;

    // Check if the associated token account exists
    let associated_token =
        spl_associated_token_account_interface::address::get_associated_token_address(
            &input.payer.pubkey(),
            &mint,
        );

    let associated_token_exists = match ctx
        .solana_client()
        .get_account_with_commitment(&associated_token, CommitmentConfig::confirmed())
        .await
    {
        Ok(response) => match response.value {
            Some(_) => Ok(true),
            None => Ok(false),
        },
        Err(_) => Err(crate::Error::AccountNotFound(associated_token)),
    }?;

    // add associated token account instruction if it doesn't exist
    let associated_token_ix =
        spl_associated_token_account_interface::instruction::create_associated_token_account(
            &input.payer.pubkey(),
            &input.payer.pubkey(),
            &mint,
            &spl_token_interface::ID,
        );

    info!("associated_token_exists: {:?}", associated_token_exists);

    info!("to: {:?}", to);
    let ix = solana_program::instruction::Instruction {
        program_id: token_bridge_program_id,
        accounts: vec![
            AccountMeta::new(input.payer.pubkey(), true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new_readonly(message, false),
            AccountMeta::new(claim_key, false),
            AccountMeta::new_readonly(endpoint, false),
            AccountMeta::new(to, false),
            match input.fee_recipient {
                Some(fee_r) => AccountMeta::new(fee_r, false),
                _ => AccountMeta::new(to, false),
            },
            AccountMeta::new(mint, false),
            AccountMeta::new_readonly(mint_meta, false),
            AccountMeta::new_readonly(mint_authority, false),
            // Dependencies
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token_interface::ID, false),
            AccountMeta::new_readonly(wormhole_core_program_id, false),
            // Program
        ],
        data: borsh::to_vec(&(
            TokenBridgeInstructions::CompleteWrapped,
            CompleteWrappedData {},
        ))?,
    };

    let instructions = if associated_token_exists {
        vec![ix]
    } else {
        vec![associated_token_ix, ix]
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer].into(),
        instructions,
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
                "mint_metadata" => mint_meta,
                "mint" => mint,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
