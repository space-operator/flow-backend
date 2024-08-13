use super::Error;
use crate::context::signer::Presigner;
use anyhow::{anyhow, bail};
use base64::prelude::*;
use nom::{
    bytes::complete::take,
    character::complete::{char, u64},
    IResult,
};
use solana_client::{
    client_error::{ClientError, ClientErrorKind},
    nonblocking::rpc_client::RpcClient,
    rpc_request::{RpcError, RpcResponseErrorData},
    rpc_response::RpcSimulateTransactionResult,
};
use solana_sdk::{
    clock::{Slot, UnixTimestamp},
    feature_set::FeatureSet,
    precompiles::verify_if_precompile,
    signature::Signature,
    transaction::Transaction,
};
use solana_transaction_status::{EncodedTransaction, TransactionBinaryEncoding};

pub fn find_failed_instruction(err: &ClientError) -> Option<usize> {
    if let ClientErrorKind::RpcError(RpcError::RpcResponseError { message, .. }) = &err.kind {
        if let Some(s) =
            message.strip_prefix("Transaction simulation failed: Error processing Instruction ")
        {
            let index = s
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>();
            index.parse().ok()
        } else {
            None
        }
    } else {
        None
    }
}

pub fn list_signatures(tx: &Transaction) -> Option<Vec<Presigner>> {
    let placeholder = Transaction::get_invalid_signature();
    let vec = tx
        .signatures
        .iter()
        .enumerate()
        .filter(|(_, &sig)| sig != placeholder)
        .map(|(index, sig)| Presigner {
            pubkey: tx.message.account_keys[index],
            signature: *sig,
        })
        .collect::<Vec<_>>();
    if vec.is_empty() {
        None
    } else {
        Some(vec)
    }
}

fn parse_rpc_memo_field_impl(mut s: &str) -> IResult<&str, Vec<String>> {
    let mut result = Vec::new();

    while !s.is_empty() {
        s = char('[')(s)?.0;
        let length;
        (s, length) = u64(s)?;
        s = char(']')(s)?.0;
        s = char(' ')(s)?.0;
        let content;
        (s, content) = take(length)(s)?;
        result.push(content.to_owned());

        if s.is_empty() {
            break;
        }

        s = char(';')(s)?.0;
        s = char(' ')(s)?.0;
    }

    Ok((s, result))
}

pub fn parse_rpc_memo_field(s: &str) -> Result<Vec<String>, anyhow::Error> {
    match parse_rpc_memo_field_impl(s) {
        Ok((_, vec)) => Ok(vec),
        Err(err) => Err(err.to_owned().into()),
    }
}

pub fn verbose_solana_error(err: &ClientError) -> String {
    use std::fmt::Write;
    if let ClientErrorKind::RpcError(RpcError::RpcResponseError {
        code,
        message,
        data,
    }) = &err.kind
    {
        let mut s = String::new();
        writeln!(s, "{} ({})", message, code).unwrap();
        if let RpcResponseErrorData::SendTransactionPreflightFailure(
            RpcSimulateTransactionResult {
                logs: Some(logs), ..
            },
        ) = data
        {
            for (i, log) in logs.iter().enumerate() {
                writeln!(s, "{}: {}", i + 1, log).unwrap();
            }
        }
        s
    } else {
        err.to_string()
    }
}

pub struct TransactionWithMeta {
    pub slot: Slot,
    pub transaction: Transaction,
    pub blocktime: Option<UnixTimestamp>,
}

pub async fn get_and_parse_transaction(
    rpc: &RpcClient,
    signature: &Signature,
) -> Result<TransactionWithMeta, anyhow::Error> {
    let result = rpc
        .get_transaction(
            &signature,
            solana_transaction_status::UiTransactionEncoding::Base64,
        )
        .await?;
    let EncodedTransaction::Binary(tx_base64, TransactionBinaryEncoding::Base64) =
        result.transaction.transaction
    else {
        return Err(anyhow!("RPC return wrong tx encoding"));
    };

    let tx_bytes = BASE64_STANDARD.decode(&tx_base64).map_err(Error::other)?;
    let tx: Transaction = bincode::deserialize(&tx_bytes).map_err(Error::other)?;

    Ok(TransactionWithMeta {
        slot: result.slot,
        transaction: tx,
        blocktime: result.block_time,
    })
}

/// Verify the precompiled programs in this transaction.
/// We make our own function because`solana-sdk`'s function return non-infomative error message.
pub fn verify_precompiles(tx: &Transaction, feature_set: &FeatureSet) -> Result<(), anyhow::Error> {
    for (index, instruction) in tx.message().instructions.iter().enumerate() {
        // The Transaction may not be sanitized at this point
        if instruction.program_id_index as usize >= tx.message().account_keys.len() {
            bail!(
                "instruction #{} error: program ID not found {}",
                index,
                instruction.program_id_index
            );
        }
        let program_id = &tx.message().account_keys[instruction.program_id_index as usize];

        verify_if_precompile(
            program_id,
            instruction,
            &tx.message().instructions,
            feature_set,
        )
        .map_err(|error| anyhow!("instruction #{} error: {}", index, error))?;
    }
    Ok(())
}
