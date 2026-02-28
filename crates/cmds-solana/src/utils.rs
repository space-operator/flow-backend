use crate::prelude::*;
use bytes::Bytes;
use rust_decimal::{
    Decimal,
    prelude::{MathematicalOps, ToPrimitive},
};
use solana_message::Message;
use solana_presigner::Presigner;
use solana_program::{hash::Hash, instruction::Instruction, native_token::LAMPORTS_PER_SOL};
use solana_transaction::Transaction;
use std::time::Duration;
use value::Error as ValueError;

pub const fn pod_get_packed_len<T: bytemuck::Pod>() -> usize {
    std::mem::size_of::<T>()
}

pub const SIGNATURE_TIMEOUT: Duration = Duration::from_secs(60 * 5);

pub async fn execute(
    client: &RpcClient,
    fee_payer: &Pubkey,
    instructions: &[Instruction],
) -> crate::Result<(Transaction, Hash)> {
    let recent_blockhash = client.get_latest_blockhash().await?;

    let message = Message::new_with_blockhash(instructions, Some(fee_payer), &recent_blockhash);

    let transaction = Transaction::new_unsigned(message);

    Ok((transaction, recent_blockhash))
}

pub async fn submit_transaction(client: &RpcClient, tx: Transaction) -> crate::Result<Signature> {
    Ok(client.send_and_confirm_transaction(&tx).await?)
}

pub fn sol_to_lamports(amount: Decimal) -> crate::Result<u64> {
    if amount < Decimal::ZERO {
        return Err(ValueError::Custom("amount is negative".into()).into());
    }
    amount
        .checked_mul(Decimal::from(LAMPORTS_PER_SOL))
        .and_then(|d| d.floor().to_u64())
        .ok_or_else(|| ValueError::Custom("value overflow".into()).into())
}

/// Convert the UI representation of a token amount (using the decimals field defined in its mint)
/// to the raw amount.
pub fn ui_amount_to_amount(ui_amount: Decimal, decimals: u8) -> crate::Result<u64> {
    if ui_amount < Decimal::ZERO {
        return Err(ValueError::Custom("amount is negative".to_owned()).into());
    }
    ui_amount
        .checked_mul(Decimal::TEN.powu(decimals as u64))
        .and_then(|d| d.floor().to_u64())
        .ok_or_else(|| ValueError::Custom("amount overflow".to_owned()).into())
}

pub fn tx_to_string(tx: &Transaction) -> Result<String, bincode::Error> {
    Ok(base64::encode(bincode::serialize(tx)?))
}

pub async fn try_sign_wallet(
    ctx: &mut CommandContext,
    tx: &mut Transaction,
    wallet: &Wallet,
    recent_blockhash: Hash,
) -> Result<(), crate::Error> {
    if let Some(keypair) = wallet.keypair() {
        tx.try_sign(&[keypair], recent_blockhash)?;
    } else {
        let msg: Bytes = tx.message_data().into();
        let sig = tokio::time::timeout(
            SIGNATURE_TIMEOUT,
            ctx.request_signature(wallet.pubkey(), None, msg.clone(), SIGNATURE_TIMEOUT),
        )
        .await
        .map_err(|_| crate::Error::SignatureTimeout)??;
        let presigner = Presigner::new(&wallet.pubkey(), &sig.signature);
        tx.try_sign(&[&presigner], recent_blockhash)?;
    }

    Ok(())
}

/// Compute the 8-byte Anchor instruction discriminator: `sha256("global:{name}")[0..8]`
pub fn anchor_discriminator(name: &str) -> [u8; 8] {
    let hash = solana_program::hash::hash(format!("global:{name}").as_bytes());
    hash.to_bytes()[..8].try_into().unwrap()
}

/// Build an Anchor instruction: 8-byte discriminator + args data.
pub fn build_anchor_instruction(
    program_id: Pubkey,
    instruction_name: &str,
    accounts: Vec<solana_program::instruction::AccountMeta>,
    args_data: Vec<u8>,
) -> Instruction {
    let mut data = Vec::with_capacity(8 + args_data.len());
    data.extend_from_slice(&anchor_discriminator(instruction_name));
    data.extend_from_slice(&args_data);
    Instruction {
        program_id,
        accounts,
        data,
    }
}
