use std::str::FromStr;

use crate::prelude::*;
use crate::streamflow::StreamContract;
use crate::utils::anchor_sighash;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_account_decoder::UiAccountEncoding;
use solana_commitment_config::CommitmentConfig;
use solana_program::instruction::AccountMeta;
use solana_rpc_client_api::config::RpcAccountInfoConfig;
use spl_associated_token_account::get_associated_token_address;
use tracing::info;

use super::{STRM_TREASURY, WithdrawData, WithdrawDataInput};

const NAME: &str = "withdraw_streamflow_timelock";

const DEFINITION: &str = flow_lib::node_definition!("streamflow/withdraw.json");

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
    fee_payer: Wallet,
    authority: Wallet,
    #[serde(with = "value::pubkey")]
    recipient: Pubkey,
    #[serde(with = "value::pubkey")]
    metadata: Pubkey,
    data: WithdrawDataInput,
    #[serde(default, with = "value::pubkey::opt")]
    partner: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

fn create_withdraw_stream_instruction(
    authority: &Pubkey,
    recipient: &Pubkey,
    recipient_tokens: &Pubkey,
    metadata: &Pubkey,
    escrow_tokens: &Pubkey,
    streamflow_treasury_tokens: &Pubkey,
    partner: &Pubkey,
    partner_tokens: &Pubkey,
    mint: &Pubkey,
    timelock_program: &Pubkey,
    data: WithdrawData,
) -> Instruction {
    let accounts = [
        AccountMeta::new(*authority, true),
        AccountMeta::new(*recipient, false),
        AccountMeta::new(*recipient_tokens, false),
        AccountMeta::new(*metadata, false),
        AccountMeta::new(*escrow_tokens, false),
        AccountMeta::new(Pubkey::from_str(STRM_TREASURY).unwrap(), false),
        AccountMeta::new(*streamflow_treasury_tokens, false),
        AccountMeta::new(*partner, false),
        AccountMeta::new(*partner_tokens, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(spl_token::ID, false),
    ]
    .to_vec();

    let discriminator = anchor_sighash("withdraw");

    Instruction {
        program_id: *timelock_program,
        accounts,
        data: (discriminator, data).try_to_vec().unwrap(),
    }
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let timelock_program = crate::streamflow::streamflow_program_id(ctx.solana_config().cluster);

    let data: WithdrawData = input.data.into();

    let commitment = CommitmentConfig::confirmed();

    let config = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        commitment: Some(commitment),
        data_slice: None,
        min_context_slot: None,
    };

    let response = ctx
        .solana_client()
        .get_account_with_config(&input.metadata, config)
        .await
        .map_err(|e| {
            tracing::error!("Error: {:?}", e);
            crate::Error::AccountNotFound(input.metadata)
        })?;

    let escrow = match response.value {
        Some(account) => account,
        None => return Err(crate::Error::AccountNotFound(input.metadata).into()),
    };

    let mut escrow_data: &[u8] = &escrow.data;
    let escrow_data = StreamContract::deserialize(&mut escrow_data).map_err(|_| {
        tracing::error!(
            "Invalid data for: {:?}",
            crate::Error::InvalidAccountData(input.metadata)
        );
        crate::Error::InvalidAccountData(input.metadata)
    })?;

    info!("Escrow account: {:?}", escrow_data);

    let recipient_tokens = get_associated_token_address(&escrow_data.recipient, &escrow_data.mint);

    let streamflow_treasury_tokens =
        get_associated_token_address(&Pubkey::from_str(STRM_TREASURY).unwrap(), &escrow_data.mint);

    let partner = match &input.partner {
        Some(partner) => *partner,
        None => Pubkey::from_str(STRM_TREASURY).unwrap(),
    };

    let partner_tokens = get_associated_token_address(&partner, &escrow_data.mint);

    let instruction = create_withdraw_stream_instruction(
        &input.fee_payer.pubkey(),
        &escrow_data.recipient,
        &recipient_tokens,
        &input.metadata,
        &escrow_data.escrow_tokens,
        &streamflow_treasury_tokens,
        &partner,
        &partner_tokens,
        &escrow_data.mint,
        &timelock_program,
        data,
    );

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

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
