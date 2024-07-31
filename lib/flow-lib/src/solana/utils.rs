use super::Error;
use anyhow::{anyhow, bail};
use base64::prelude::*;
use solana_client::{
    client_error::{ClientError, ClientErrorKind},
    nonblocking::rpc_client::RpcClient,
    rpc_request::{RpcError, RpcResponseErrorData},
    rpc_response::RpcSimulateTransactionResult,
};
use solana_sdk::{
    feature_set::FeatureSet, precompiles::verify_if_precompile, signature::Signature,
    transaction::Transaction,
};
use solana_transaction_status::{
    extract_memos::ExtractMemos, EncodedTransaction, TransactionBinaryEncoding,
};

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

pub async fn get_memos(
    rpc: &RpcClient,
    signature: &Signature,
) -> Result<Vec<String>, anyhow::Error> {
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
    let memos = tx.message.extract_memos();

    Ok(memos)
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
