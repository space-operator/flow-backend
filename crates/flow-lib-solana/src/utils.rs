use super::Error;
use super::{Pubkey, Signature};
use flow_lib::context::signer::Presigner;
use flow_lib::utils::tower_client::CommonErrorExt;
use agave_feature_set::FeatureSet;
use agave_precompiles::verify_if_precompile;
use anyhow::{anyhow, bail};
use base64::prelude::*;
use nom::{
    IResult,
    bytes::complete::take,
    character::complete::{char, u64},
};
use solana_address_lookup_table_interface::state::AddressLookupTable;
use solana_clock::{Slot, UnixTimestamp};
use solana_program::message::AddressLookupTableAccount;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::{
    client_error::{Error as ClientError, ErrorKind as ClientErrorKind},
    request::{RpcError, RpcResponseErrorData},
    response::RpcSimulateTransactionResult,
};
use solana_transaction::{Transaction, versioned::VersionedTransaction};
use solana_transaction_status::{EncodedTransaction, TransactionBinaryEncoding};

pub async fn fetch_address_lookup_table(
    rpc: &RpcClient,
    pubkey: &Pubkey,
) -> Result<AddressLookupTableAccount, Error> {
    let raw_account = rpc
        .get_account(pubkey)
        .await
        .map_err(|error| Error::solana(error, 0))?;
    let table = AddressLookupTable::deserialize(&raw_account.data)?;
    Ok(AddressLookupTableAccount {
        key: *pubkey,
        addresses: table.addresses.to_vec(),
    })
}

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

pub fn list_signatures(tx: &VersionedTransaction) -> Option<Vec<Presigner>> {
    let placeholder = Transaction::get_invalid_signature();
    let accounts = tx.message.static_account_keys();
    let vec = tx
        .signatures
        .iter()
        .enumerate()
        .filter(|(_, sig)| **sig != placeholder)
        .map(|(index, sig)| Presigner {
            pubkey: accounts[index],
            signature: *sig,
        })
        .collect::<Vec<_>>();
    if vec.is_empty() { None } else { Some(vec) }
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
            signature,
            solana_transaction_status::UiTransactionEncoding::Base64,
        )
        .await?;
    let EncodedTransaction::Binary(tx_base64, TransactionBinaryEncoding::Base64) =
        result.transaction.transaction
    else {
        return Err(anyhow!("RPC return wrong tx encoding"));
    };

    let tx_bytes = BASE64_STANDARD.decode(&tx_base64).map_err(Error::other)?;
    let tx: Transaction = bincode1::deserialize(&tx_bytes).map_err(Error::other)?;

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
