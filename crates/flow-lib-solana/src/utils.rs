use std::collections::BTreeSet;
use std::sync::Arc;

use crate::InstructionsExt;

use super::Error;
use super::{Pubkey, Signature};
use agave_feature_set::FeatureSet;
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
use solana_message::{
    AddressLookupTableAccount, VersionedMessage, compiled_instruction::CompiledInstruction, v0,
};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::{
    client_error::{Error as ClientError, ErrorKind as ClientErrorKind},
    request::RpcError,
};
use solana_transaction::{Transaction, versioned::VersionedTransaction};
use solana_transaction_status::{EncodedTransaction, TransactionBinaryEncoding};
use spo_helius::{
    GetPriorityFeeEstimateOptions, GetPriorityFeeEstimateRequest, Helius, PriorityLevel,
};

pub async fn get_priority_fee(
    helius: &Helius,
    accounts: &BTreeSet<Pubkey>,
) -> Result<u64, anyhow::Error> {
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
}

pub fn simple_execute_svc(
    rpc: Arc<RpcClient>,
    helius: Option<Arc<Helius>>,
    network: SolanaNet,
    signer: signer::Svc,
    flow_run_id: Option<FlowRunId>,
    config: ExecutionConfig,
) -> execute::Svc {
    let handle = move |req: execute::Request| {
        let rpc = rpc.clone();
        let signer = signer.clone();
        let config = config.clone();
        let helius = helius.clone();
        async move {
            Ok(execute::Response {
                signature: Some(
                    req.instructions
                        .execute(
                            &rpc,
                            helius.as_deref(),
                            network,
                            signer,
                            flow_run_id,
                            config,
                        )
                        .await?,
                ),
            })
        }
    };
    execute::Svc::new(tower::service_fn(handle))
}

/// Creates a [`CommandContext`] with a real execute service for integration tests.
///
/// Uses `simple_execute_svc` pointed at devnet with default config.
/// All other services (signer, get_jwt, api_input) remain unimplemented.
pub fn test_context_with_execute() -> flow_lib::context::CommandContext {
    use flow_lib::context::{
        CommandContext, CommandContextData, FlowContextData, FlowServices, FlowSetContextData,
        FlowSetServices, get_jwt,
    };
    use flow_lib::flow_run_events;
    use flow_lib::utils::tower_client::unimplemented_svc;
    use flow_lib::{ContextConfig, NodeId};
    use std::collections::HashMap;

    let config = ContextConfig::default();
    let solana_client = Arc::new(config.solana_client.build_client(None));
    let node_id = NodeId::nil();
    let times = 0;
    let (tx, _) = flow_run_events::channel();

    let execute_svc = simple_execute_svc(
        solana_client.clone(),
        None,
        SolanaNet::Devnet,
        unimplemented_svc(),
        None,
        ExecutionConfig::default(),
    );

    CommandContext::builder()
        .execute(execute_svc)
        .get_jwt(unimplemented_svc::<
            get_jwt::Request,
            get_jwt::Response,
            get_jwt::Error,
        >())
        .flow(
            FlowServices::builder()
                .signer(unimplemented_svc())
                .set(
                    FlowSetServices::builder()
                        .http(reqwest::Client::new())
                        .solana_client(solana_client)
                        .extensions(Default::default())
                        .api_input(unimplemented_svc())
                        .build(),
                )
                .build(),
        )
        .data(CommandContextData {
            node_id,
            times,
            flow: FlowContextData {
                flow_run_id: FlowRunId::nil(),
                environment: HashMap::new(),
                inputs: Default::default(),
                read_only: false,
                set: FlowSetContextData {
                    flow_owner: Default::default(),
                    started_by: Default::default(),
                    endpoints: Default::default(),
                    solana: config.solana_client,
                    http: config.http_client,
                },
            },
        })
        .node_log(flow_run_events::NodeLogSender::new(tx, node_id, times))
        .build()
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
    if let ClientErrorKind::RpcError(RpcError::RpcResponseError { message, .. }) = &*err.kind {
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

        #[allow(deprecated)]
        agave_precompiles::verify_if_precompile(
            program_id,
            instruction,
            &tx.message().instructions,
            feature_set,
        )
        .map_err(|error| anyhow!("instruction #{} error: {}", index, error))?;
    }
    Ok(())
}

const SWIG_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("swigypWHEksbC64pWKwah1WTeh9JXwx8H1rJHLdbQMB");
const COMPUTE_BUDGET_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("ComputeBudget111111111111111111111111111111");
const LIGHTHOUSE_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("L2TExMFKdjpN9kozasaurPirfHy9P8sbXoAN1qA3S95");
const SPL_MEMO_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
const SPL_MEMO_LEGACY_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo");

/// Check if a message references the Swig program in any of its instructions.
fn contains_swig_program(msg: &v0::Message) -> bool {
    msg.instructions.iter().any(|ix| {
        msg.account_keys
            .get(ix.program_id_index as usize)
            .map(|pk| *pk == SWIG_PROGRAM_ID)
            .unwrap_or(false)
    })
}

fn instruction_program(
    msg: &v0::Message,
    ix: &CompiledInstruction,
) -> Result<Pubkey, anyhow::Error> {
    msg.account_keys
        .get(ix.program_id_index as usize)
        .copied()
        .ok_or_else(|| anyhow!("instruction program index out of bounds"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AccountIdentity {
    Static(Pubkey),
    Lookup { table: Pubkey, address_index: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InstructionAccount {
    identity: AccountIdentity,
    is_signer: bool,
    is_writable: bool,
}

fn account_identity(msg: &v0::Message, index: usize) -> Result<AccountIdentity, anyhow::Error> {
    if let Some(pubkey) = msg.account_keys.get(index) {
        return Ok(AccountIdentity::Static(*pubkey));
    }

    let mut offset = index
        .checked_sub(msg.account_keys.len())
        .ok_or_else(|| anyhow!("instruction account index out of bounds"))?;

    for lookup in &msg.address_table_lookups {
        for address_index in &lookup.writable_indexes {
            if offset == 0 {
                return Ok(AccountIdentity::Lookup {
                    table: lookup.account_key,
                    address_index: *address_index,
                });
            }
            offset -= 1;
        }
    }

    for lookup in &msg.address_table_lookups {
        for address_index in &lookup.readonly_indexes {
            if offset == 0 {
                return Ok(AccountIdentity::Lookup {
                    table: lookup.account_key,
                    address_index: *address_index,
                });
            }
            offset -= 1;
        }
    }

    Err(anyhow!("instruction account index out of bounds"))
}

fn instruction_accounts(
    msg: &v0::Message,
    ix: &CompiledInstruction,
) -> Result<Vec<InstructionAccount>, anyhow::Error> {
    ix.accounts
        .iter()
        .map(|index| {
            let index = *index as usize;
            let identity = account_identity(msg, index)?;
            let is_signer = index < msg.header.num_required_signatures as usize;
            let is_writable = msg.is_maybe_writable(index, None);
            Ok(InstructionAccount {
                identity,
                is_signer,
                is_writable,
            })
        })
        .collect()
}

fn is_allowed_compute_budget_instruction(
    msg: &v0::Message,
    ix: &CompiledInstruction,
) -> Result<bool, anyhow::Error> {
    if instruction_program(msg, ix)? != COMPUTE_BUDGET_PROGRAM_ID {
        return Ok(false);
    }
    ensure!(
        ix.accounts.is_empty(),
        "compute budget instruction must not include accounts"
    );
    let Some((&tag, rest)) = ix.data.split_first() else {
        return Ok(true);
    };
    let allowed = match (tag, rest.len()) {
        // RequestUnitsDeprecated { units: u32, additional_fee: u32 }
        (0, 8) => {
            let units = u32::from_le_bytes(rest[0..4].try_into().unwrap());
            let additional_fee = u32::from_le_bytes(rest[4..8].try_into().unwrap());
            units <= 1_400_000 && additional_fee <= 1_500_000
        }
        // RequestHeapFrame(u32)
        (1, 4) => {
            let heap_bytes = u32::from_le_bytes(rest.try_into().unwrap());
            heap_bytes <= 256 * 1024
        }
        // SetComputeUnitLimit(u32)
        (2, 4) => {
            let units = u32::from_le_bytes(rest.try_into().unwrap());
            units <= 1_400_000
        }
        // SetComputeUnitPrice(u64)
        (3, 8) => {
            let micro_lamports = u64::from_le_bytes(rest.try_into().unwrap());
            micro_lamports <= 1_000_000
        }
        // SetLoadedAccountsDataSizeLimit(u32)
        (4, 4) => {
            let bytes = u32::from_le_bytes(rest.try_into().unwrap());
            bytes <= 1_048_576
        }
        _ => rest.len() <= 16,
    };
    Ok(allowed)
}

fn is_allowed_wallet_added_instruction(
    original: &v0::Message,
    modified: &v0::Message,
    ix: &CompiledInstruction,
) -> Result<bool, anyhow::Error> {
    if is_allowed_compute_budget_instruction(modified, ix)? {
        return Ok(true);
    }

    let program = instruction_program(modified, ix)?;
    let original_static_accounts = original
        .account_keys
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let original_signers = original
        .account_keys
        .iter()
        .take(original.header.num_required_signatures as usize)
        .copied()
        .collect::<BTreeSet<_>>();
    let is_safe_wallet_annotation_account = |account: &InstructionAccount| {
        let AccountIdentity::Static(pubkey) = account.identity else {
            return false;
        };
        if account.is_signer {
            return original_signers.contains(&pubkey);
        }
        !original_static_accounts.contains(&pubkey)
    };

    if program == SPL_MEMO_PROGRAM_ID || program == SPL_MEMO_LEGACY_PROGRAM_ID {
        let accounts = instruction_accounts(modified, ix)?;
        return Ok(ix.data.len() <= 256
            && accounts
                .iter()
                .all(|account| is_safe_wallet_annotation_account(account)));
    }

    if program == LIGHTHOUSE_PROGRAM_ID {
        if ix.data.len() > 512 || ix.accounts.len() > 8 {
            return Ok(false);
        }
        let accounts = instruction_accounts(modified, ix)?;
        return Ok(accounts.iter().all(|account| {
            let AccountIdentity::Static(pubkey) = account.identity else {
                return false;
            };
            !account.is_signer || original_signers.contains(&pubkey)
        }));
    }

    Ok(false)
}

fn business_instructions<'a>(
    msg: &'a v0::Message,
) -> Result<Vec<&'a CompiledInstruction>, anyhow::Error> {
    msg.instructions
        .iter()
        .filter_map(|ix| match is_allowed_compute_budget_instruction(msg, ix) {
            Ok(true) => None,
            Ok(false) => Some(Ok(ix)),
            Err(error) => Some(Err(error)),
        })
        .collect()
}

fn business_instructions_allowing_wallet_additions<'a>(
    original: &v0::Message,
    modified: &'a v0::Message,
) -> Result<Vec<&'a CompiledInstruction>, anyhow::Error> {
    modified
        .instructions
        .iter()
        .filter_map(
            |ix| match is_allowed_wallet_added_instruction(original, modified, ix) {
                Ok(true) => None,
                Ok(false) => Some(Ok(ix)),
                Err(error) => Some(Err(error)),
            },
        )
        .collect()
}

fn instruction_program_labels(
    msg: &v0::Message,
    instructions: &[&CompiledInstruction],
) -> Result<Vec<String>, anyhow::Error> {
    instructions
        .iter()
        .map(|ix| instruction_program(msg, ix).map(|program| program.to_string()))
        .collect()
}

/// Validate that a modified message is compatible with the original.
///
/// Always checks fee payer and blockhash, then compares business
/// instructions while allowing narrowly-scoped wallet-added instructions.
/// Swig instructions are still compared normally; the presence of Swig must
/// not bypass validation of the compiled instruction data and accounts.
///
/// `l` is old, `r` is new.
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
    ensure!(!l.account_keys.is_empty(), "empty transaction");
    ensure!(!r.account_keys.is_empty(), "empty transaction");
    ensure!(
        l.account_keys[0] == r.account_keys[0],
        "different fee payer"
    );
    ensure!(
        l.recent_blockhash == r.recent_blockhash,
        "different blockhash"
    );
    ensure!(
        l.address_table_lookups == r.address_table_lookups,
        "different address table lookups"
    );

    ensure!(
        contains_swig_program(&l) == contains_swig_program(&r),
        "swig program presence changed"
    );

    ensure!(
        l.header.num_required_signatures == r.header.num_required_signatures,
        "different num_required_signatures"
    );
    ensure!(
        l.header.num_readonly_signed_accounts == r.header.num_readonly_signed_accounts,
        "different num_readonly_signed_accounts"
    );
    for i in 0..l.header.num_required_signatures as usize {
        ensure!(
            l.account_keys.get(i) == r.account_keys.get(i),
            "different signer account {}",
            i
        );
    }

    let l_business = business_instructions(&l)?;
    let r_business = business_instructions_allowing_wallet_additions(&l, &r)?;
    if l_business.len() != r_business.len() {
        let old_programs = instruction_program_labels(&l, &l_business)?.join(",");
        let new_programs = instruction_program_labels(&r, &r_business)?.join(",");
        bail!(
            "different business instructions count, old = {}, new = {}, old_programs = [{}], new_programs = [{}]",
            l_business.len(),
            r_business.len(),
            old_programs,
            new_programs,
        );
    }

    for (i, (il, ir)) in l_business.iter().zip(r_business.iter()).enumerate() {
        ensure!(
            instruction_program(&l, il)? == instruction_program(&r, ir)?,
            "different program id for instruction {}",
            i
        );
        ensure!(il.data == ir.data, "different instruction data {}", i);
        ensure!(
            instruction_accounts(&l, il)? == instruction_accounts(&r, ir)?,
            "different account inputs for instruction {}",
            i
        );
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use solana_message::MessageHeader;

    fn serialize_message(message: v0::Message) -> Vec<u8> {
        bincode1::serialize(&VersionedMessage::V0(message)).unwrap()
    }

    fn fee_payer() -> Pubkey {
        solana_pubkey::pubkey!("11111111111111111111111111111112")
    }

    fn program() -> Pubkey {
        solana_pubkey::pubkey!("11111111111111111111111111111113")
    }

    fn lookup_table() -> Pubkey {
        solana_pubkey::pubkey!("11111111111111111111111111111114")
    }

    #[test]
    fn same_message_logic_rejects_swig_data_rewrite() {
        let original = v0::Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![fee_payer(), SWIG_PROGRAM_ID],
            recent_blockhash: Default::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 1,
                accounts: vec![],
                data: vec![1],
            }],
            address_table_lookups: vec![],
        };
        let mut modified = original.clone();
        modified.instructions[0].data = vec![2];

        let error =
            is_same_message_logic(&serialize_message(original), &serialize_message(modified))
                .unwrap_err()
                .to_string();
        assert!(error.contains("different instruction data"));
    }

    #[test]
    fn same_message_logic_compares_lookup_accounts_after_static_account_insert() {
        let lookup = v0::MessageAddressTableLookup {
            account_key: lookup_table(),
            writable_indexes: vec![7],
            readonly_indexes: vec![],
        };
        let original = v0::Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![fee_payer(), program()],
            recent_blockhash: Default::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 1,
                accounts: vec![2],
                data: vec![1],
            }],
            address_table_lookups: vec![lookup.clone()],
        };
        let modified = v0::Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 2,
            },
            account_keys: vec![
                fee_payer(),
                program(),
                solana_pubkey::pubkey!("11111111111111111111111111111115"),
            ],
            recent_blockhash: Default::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 1,
                accounts: vec![3],
                data: vec![1],
            }],
            address_table_lookups: vec![lookup],
        };

        is_same_message_logic(&serialize_message(original), &serialize_message(modified)).unwrap();
    }

    #[test]
    fn same_message_logic_rejects_different_lookup_account() {
        let lookup = v0::MessageAddressTableLookup {
            account_key: lookup_table(),
            writable_indexes: vec![7, 8],
            readonly_indexes: vec![],
        };
        let original = v0::Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![fee_payer(), program()],
            recent_blockhash: Default::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 1,
                accounts: vec![2],
                data: vec![1],
            }],
            address_table_lookups: vec![lookup.clone()],
        };
        let mut modified = original.clone();
        modified.instructions[0].accounts = vec![3];

        let error =
            is_same_message_logic(&serialize_message(original), &serialize_message(modified))
                .unwrap_err()
                .to_string();
        assert!(error.contains("different account inputs"));
    }

    #[test]
    fn same_message_logic_allows_wallet_added_lighthouse_signer_annotation() {
        let original = v0::Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![fee_payer(), program()],
            recent_blockhash: Default::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 1,
                accounts: vec![],
                data: vec![1],
            }],
            address_table_lookups: vec![],
        };
        let modified = v0::Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 2,
            },
            account_keys: vec![fee_payer(), program(), LIGHTHOUSE_PROGRAM_ID],
            recent_blockhash: Default::default(),
            instructions: vec![
                CompiledInstruction {
                    program_id_index: 1,
                    accounts: vec![],
                    data: vec![1],
                },
                CompiledInstruction {
                    program_id_index: 2,
                    accounts: vec![0],
                    data: vec![1],
                },
            ],
            address_table_lookups: vec![],
        };

        is_same_message_logic(&serialize_message(original), &serialize_message(modified)).unwrap();
    }

    #[test]
    fn same_message_logic_allows_wallet_added_lighthouse_original_account_reference() {
        let original = v0::Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![fee_payer(), lookup_table(), program()],
            recent_blockhash: Default::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 2,
                accounts: vec![1],
                data: vec![1],
            }],
            address_table_lookups: vec![],
        };
        let modified = v0::Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 2,
            },
            account_keys: vec![
                fee_payer(),
                lookup_table(),
                program(),
                LIGHTHOUSE_PROGRAM_ID,
            ],
            recent_blockhash: Default::default(),
            instructions: vec![
                CompiledInstruction {
                    program_id_index: 2,
                    accounts: vec![1],
                    data: vec![1],
                },
                CompiledInstruction {
                    program_id_index: 3,
                    accounts: vec![1],
                    data: vec![1],
                },
            ],
            address_table_lookups: vec![],
        };

        is_same_message_logic(&serialize_message(original), &serialize_message(modified)).unwrap();
    }

    #[test]
    fn same_message_logic_rejects_wallet_added_lighthouse_extra_signer() {
        let extra_signer = solana_pubkey::pubkey!("11111111111111111111111111111116");
        let original = v0::Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![fee_payer(), program()],
            recent_blockhash: Default::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 1,
                accounts: vec![],
                data: vec![1],
            }],
            address_table_lookups: vec![],
        };
        let modified = v0::Message {
            header: MessageHeader {
                num_required_signatures: 2,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 2,
            },
            account_keys: vec![fee_payer(), extra_signer, program(), LIGHTHOUSE_PROGRAM_ID],
            recent_blockhash: Default::default(),
            instructions: vec![
                CompiledInstruction {
                    program_id_index: 2,
                    accounts: vec![],
                    data: vec![1],
                },
                CompiledInstruction {
                    program_id_index: 3,
                    accounts: vec![1],
                    data: vec![1],
                },
            ],
            address_table_lookups: vec![],
        };

        let error =
            is_same_message_logic(&serialize_message(original), &serialize_message(modified))
                .unwrap_err()
                .to_string();
        assert!(error.contains("different num_required_signatures"));
    }

    #[test]
    fn same_message_logic_allows_wallet_added_unknown_compute_budget_instruction() {
        let original = v0::Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![fee_payer(), program()],
            recent_blockhash: Default::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 1,
                accounts: vec![],
                data: vec![1],
            }],
            address_table_lookups: vec![],
        };
        let modified = v0::Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 2,
            },
            account_keys: vec![fee_payer(), program(), COMPUTE_BUDGET_PROGRAM_ID],
            recent_blockhash: Default::default(),
            instructions: vec![
                CompiledInstruction {
                    program_id_index: 2,
                    accounts: vec![],
                    data: vec![9, 1, 2, 3, 4],
                },
                CompiledInstruction {
                    program_id_index: 1,
                    accounts: vec![],
                    data: vec![1],
                },
            ],
            address_table_lookups: vec![],
        };

        is_same_message_logic(&serialize_message(original), &serialize_message(modified)).unwrap();
    }

    #[test]
    fn same_message_logic_allows_wallet_added_memo_signer_annotation() {
        let original = v0::Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![fee_payer(), program()],
            recent_blockhash: Default::default(),
            instructions: vec![CompiledInstruction {
                program_id_index: 1,
                accounts: vec![],
                data: vec![1],
            }],
            address_table_lookups: vec![],
        };
        let modified = v0::Message {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 2,
            },
            account_keys: vec![fee_payer(), program(), SPL_MEMO_PROGRAM_ID],
            recent_blockhash: Default::default(),
            instructions: vec![
                CompiledInstruction {
                    program_id_index: 1,
                    accounts: vec![],
                    data: vec![1],
                },
                CompiledInstruction {
                    program_id_index: 2,
                    accounts: vec![0],
                    data: b"wallet memo".to_vec(),
                },
            ],
            address_table_lookups: vec![],
        };

        is_same_message_logic(&serialize_message(original), &serialize_message(modified)).unwrap();
    }
}
