use std::collections::BTreeSet;
use std::sync::{Arc, LazyLock};

use crate::InstructionsExt;

use super::Error;
use super::{Pubkey, Signature};
use agave_feature_set::FeatureSet;
use agave_precompiles::verify_if_precompile;
use anyhow::{anyhow, bail, ensure};
use base64::prelude::*;
use flow_lib::context::execute;
use flow_lib::context::signer::{self, Presigner};
use flow_lib::solana::ExecutionConfig;
use flow_lib::utils::tower_client::CommonErrorExt;
use flow_lib::{FlowRunId, SolanaNet};
use nom::{
    IResult,
    bytes::complete::take,
    character::complete::{char, u64},
};
use solana_address_lookup_table_interface::state::AddressLookupTable;
use solana_clock::{Slot, UnixTimestamp};
use solana_program::message::{AddressLookupTableAccount, VersionedMessage, v0};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::{
    client_error::{Error as ClientError, ErrorKind as ClientErrorKind},
    request::RpcError,
};
use solana_transaction::{Transaction, versioned::VersionedTransaction};
use solana_transaction_status::{EncodedTransaction, TransactionBinaryEncoding};
use spo_helius::{GetPriorityFeeEstimateOptions, GetPriorityFeeEstimateRequest, Helius, PriorityLevel};

pub async fn get_priority_fee(accounts: &BTreeSet<Pubkey>) -> Result<u64, anyhow::Error> {
    static HTTP: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);
    if let Ok(apikey) = std::env::var("HELIUS_API_KEY") {
        let helius = Helius::new(HTTP.clone(), &apikey);
        // Not available on devnet and testnet
        let network = SolanaNet::Mainnet;
        let resp = helius
            .get_priority_fee_estimate(
                network.as_str(),
                GetPriorityFeeEstimateRequest {
                    account_keys: Some(accounts.iter().map(|pk| pk.to_string()).collect()),
                    options: Some(GetPriorityFeeEstimateOptions {
                        priority_level: Some(PriorityLevel::Medium),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .await?;
        tracing::debug!("helius response: {:?}", resp);
        Ok(resp
            .priority_fee_estimate
            .ok_or_else(|| anyhow!("helius didn't return fee"))?
            .round() as u64)
    } else {
        bail!("no HELIUS_API_KEY env");
    }
}


pub fn simple_execute_svc(
    rpc: Arc<RpcClient>,
    network: SolanaNet,
    signer: signer::Svc,
    flow_run_id: Option<FlowRunId>,
    config: ExecutionConfig,
) -> execute::Svc {
    let handle = move |req: execute::Request| {
        let rpc = rpc.clone();
        let signer = signer.clone();
        let config = config.clone();
        async move {
            Ok(execute::Response {
                signature: Some(
                    req.instructions
                        .execute(&rpc, network, signer, flow_run_id, config)
                        .await?,
                ),
            })
        }
    };
    execute::Svc::new(tower::service_fn(handle))
}

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

/// `l` is old, `r` is new
pub fn is_same_message_logic(l: &[u8], r: &[u8]) -> Result<v0::Message, anyhow::Error> {
    let l = bincode1::deserialize::<VersionedMessage>(l)?;
    let l = if let VersionedMessage::V0(l) = l {
        l
    } else {
        return Err(anyhow!("only V0 message is supported"));
    };
    let r = bincode1::deserialize::<VersionedMessage>(r)?;
    let r = if let VersionedMessage::V0(r) = r {
        r
    } else {
        return Err(anyhow!("only V0 message is supported"));
    };
    l.sanitize()?;
    r.sanitize()?;
    ensure!(
        l.header.num_required_signatures == r.header.num_required_signatures,
        "different num_required_signatures"
    );
    ensure!(
        l.header.num_readonly_signed_accounts >= r.header.num_readonly_signed_accounts,
        "different num_readonly_signed_accounts"
    );
    if l.header.num_readonly_signed_accounts != r.header.num_readonly_signed_accounts {
        tracing::warn!(
            "less num_readonly_signed_accounts, old = {}, new = {}",
            l.header.num_readonly_signed_accounts,
            r.header.num_readonly_signed_accounts
        );
    }
    ensure!(
        l.header.num_readonly_unsigned_accounts >= r.header.num_readonly_unsigned_accounts,
        "different num_readonly_unsigned_accounts"
    );
    if l.header.num_readonly_unsigned_accounts != r.header.num_readonly_unsigned_accounts {
        tracing::warn!(
            "less num_readonly_unsigned_accounts, old = {}, new = {}",
            l.header.num_readonly_unsigned_accounts,
            r.header.num_readonly_unsigned_accounts
        );
    }
    ensure!(
        l.account_keys.len() == r.account_keys.len(),
        "different account inputs length, old = {}, new = {}",
        l.account_keys.len(),
        r.account_keys.len()
    );
    ensure!(!l.account_keys.is_empty(), "empty transaction");
    ensure!(
        l.instructions.len() == r.instructions.len(),
        "different instructions count, old = {}, new = {}",
        l.instructions.len(),
        r.instructions.len()
    );
    ensure!(
        l.account_keys[0] == r.account_keys[0],
        "different fee payer"
    );
    ensure!(
        l.recent_blockhash == r.recent_blockhash,
        "different blockhash"
    );

    /*
     * TODO
    for i in 0..l.instructions.len() {
        let program_id_l = l
            .program_id(i)
            .ok_or_else(|| anyhow!("no program id for instruction {}", i))?;

        let program_id_r = r
            .program_id(i)
            .ok_or_else(|| anyhow!("no program id for instruction {}", i))?;
        ensure!(
            program_id_l == program_id_r,
            "different program id for instruction {}",
            i
        );
        let il = &l.instructions[i];
        let ir = &r.instructions[i];
        ensure!(il.data == ir.data, "different instruction data {}", i);
        let inputs_l = il.accounts.iter().map(|i| l.account_keys.get(*i as usize));
        let inputs_r = ir.accounts.iter().map(|i| r.account_keys.get(*i as usize));
        inputs_l
            .zip(inputs_r)
            .map(|(l, r)| {
                (l == r)
                    .then_some(())
                    .ok_or_else(|| anyhow!("different account inputs for instruction {}", i))
            })
            .collect::<Result<Vec<()>, _>>()?;
    }
    */

    Ok(r)
}

#[derive(Debug)]
pub struct ParsedMemo {
    pub identity: Pubkey,
    pub timestamp: i64,
    pub run_id: FlowRunId,
}

pub fn parse_action_memo(reference: &str) -> Result<ParsedMemo, anyhow::Error> {
    let mut parts = reference.split(':');
    let scheme = parts.next();
    ensure!(scheme == Some("solana-action"), "scheme != solana-action");

    let identity: Pubkey = parts
        .next()
        .ok_or_else(|| anyhow!("no identity pubkey"))?
        .parse()?;

    let reference = parts.next().ok_or_else(|| anyhow!("no reference"))?;
    let reference = bs58::decode(reference).into_vec()?;
    ensure!(reference.len() == 32, "decoded length != 32");

    let signature: Signature = parts
        .next()
        .ok_or_else(|| anyhow!("no signature"))?
        .parse()?;

    ensure!(
        signature.verify(&identity.to_bytes(), &reference),
        "signature verification failed"
    );

    let timestamp = i64::from_le_bytes(reference[0..size_of::<i64>()].try_into().unwrap());
    let run_id = FlowRunId::from_slice(&reference[size_of::<i64>()..(size_of::<i64>() + 16)])?;
    Ok(ParsedMemo {
        identity,
        timestamp,
        run_id,
    })
}
