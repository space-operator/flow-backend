use crate::prelude::*;
use bytes::Bytes;
use futures::{
    stream::{FuturesUnordered, TryStreamExt},
    TryFutureExt,
};
use rust_decimal::{
    prelude::{MathematicalOps, ToPrimitive},
    Decimal,
};
use solana_program::{
    hash::Hash, instruction::Instruction, message::Message, native_token::LAMPORTS_PER_SOL,
};
use solana_sdk::{signature::Presigner, transaction::Transaction};
use std::{collections::BTreeSet, time::Duration};
use value::Error as ValueError;

pub const SIGNATURE_TIMEOUT: Duration = Duration::from_secs(60 * 5);

pub async fn execute(
    client: &RpcClient,
    fee_payer: &Pubkey,
    instructions: &[Instruction],
    minimum_balance_for_rent_exemption: u64,
) -> crate::Result<(Transaction, Hash)> {
    let recent_blockhash = client.get_latest_blockhash().await?;

    let message = Message::new_with_blockhash(instructions, Some(fee_payer), &recent_blockhash);

    let balance = client.get_balance(fee_payer).await?;

    let needed = minimum_balance_for_rent_exemption + client.get_fee_for_message(&message).await?;

    if balance < needed {
        return Err(crate::Error::InsufficientSolanaBalance { balance, needed });
    }

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
    ctx: &Context,
    tx: &mut Transaction,
    keypairs: &[&Keypair],
    recent_blockhash: Hash,
) -> Result<(), crate::Error> {
    let msg: Bytes = tx.message_data().into();

    let futs = keypairs
        .iter()
        .filter(|&k| k.is_user_wallet())
        .map(|k| k.pubkey())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|pk| {
            ctx.request_signature(pk, msg.clone(), SIGNATURE_TIMEOUT)
                .map_ok(move |sig| (pk, sig))
        })
        .collect::<FuturesUnordered<_>>();

    let presigners = tokio::time::timeout(SIGNATURE_TIMEOUT, futs.try_collect::<Vec<_>>())
        .await
        .map_err(|_| crate::Error::SignatureTimeout)??
        .into_iter()
        .map(|(pk, sig)| Presigner::new(&pk, &sig))
        .collect::<Vec<Presigner>>();

    let mut signers = Vec::<&dyn Signer>::with_capacity(keypairs.len());

    for p in &presigners {
        signers.push(p);
    }

    for k in keypairs {
        if !k.is_user_wallet() {
            signers.push(*k);
        }
    }

    tx.try_sign(&signers, recent_blockhash)?;

    Ok(())
}

//
pub fn anchor_sighash(name: &str) -> [u8; 8] {
    let namespace = "global";
    let preimage = format!("{}:{}", namespace, name);
    let mut sighash = [0u8; 8];
    sighash.copy_from_slice(
        &anchor_lang::solana_program::hash::hash(preimage.as_bytes()).to_bytes()[..8],
    );
    sighash
}
