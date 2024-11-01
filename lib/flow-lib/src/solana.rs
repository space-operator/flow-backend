use crate::{
    context::{execute::Error, signer},
    FlowRunId, SolanaNet,
};
use anyhow::{anyhow, bail, ensure};
use borsh::BorshDeserialize;
use bytes::Bytes;
use chrono::Utc;
use futures::TryStreamExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, serde_conv, DisplayFromStr};
use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::SerializableTransaction,
    rpc_config::RpcSendTransactionConfig,
};
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    compute_budget::{self, ComputeBudgetInstruction},
    feature_set::FeatureSet,
    instruction::{AccountMeta, CompiledInstruction, Instruction},
    message::Message,
    sanitize::Sanitize,
    signature::Presigner,
    signer::Signer,
    transaction::Transaction,
};
use spo_helius::{
    GetPriorityFeeEstimateOptions, GetPriorityFeeEstimateRequest, Helius, PriorityLevel,
};
use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    convert::Infallible,
    fmt::Display,
    num::ParseIntError,
    str::FromStr,
    time::Duration,
};
use tower::ServiceExt;
use value::{
    with::{AsKeypair, AsPubkey},
    Value,
};

pub const SIGNATURE_TIMEOUT: Duration = Duration::from_secs(3 * 60);

pub use solana_sdk::pubkey::Pubkey;
pub use solana_sdk::signature::Signature;
pub use solana_sdk::signer::keypair::Keypair;

pub mod utils;
pub use utils::*;

pub mod watcher;
pub use watcher::*;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum Wallet {
    Keypair(#[serde_as(as = "AsKeypair")] Keypair),
    Adapter {
        #[serde_as(as = "AsPubkey")]
        public_key: Pubkey,
    },
}

impl From<Keypair> for Wallet {
    fn from(value: Keypair) -> Self {
        Self::Keypair(value)
    }
}

impl Clone for Wallet {
    fn clone(&self) -> Self {
        match self {
            Wallet::Keypair(keypair) => Wallet::Keypair(keypair.clone_keypair()),
            Wallet::Adapter { public_key } => Wallet::Adapter {
                public_key: public_key.clone(),
            },
        }
    }
}

impl Wallet {
    pub fn is_adapter_wallet(&self) -> bool {
        matches!(self, Wallet::Adapter { .. })
    }

    pub fn pubkey(&self) -> Pubkey {
        match self {
            Wallet::Keypair(keypair) => keypair.pubkey(),
            Wallet::Adapter { public_key, .. } => public_key.clone(),
        }
    }

    pub fn keypair(&self) -> Option<&Keypair> {
        match self {
            Wallet::Keypair(keypair) => Some(keypair),
            Wallet::Adapter { .. } => None,
        }
    }
}

/// `l` is old, `r` is new
pub fn is_same_message_logic(l: &[u8], r: &[u8]) -> Result<Message, anyhow::Error> {
    let l = bincode::deserialize::<Message>(l)?;
    let r = bincode::deserialize::<Message>(r)?;
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

    Ok(r)
}

pub trait KeypairExt: Sized {
    fn from_str(s: &str) -> Result<Self, anyhow::Error>;
    fn new_adapter_wallet(pk: Pubkey) -> Self;
    fn clone_keypair(&self) -> Self;
    fn is_adapter_wallet(&self) -> bool;
}

impl KeypairExt for Keypair {
    fn from_str(s: &str) -> Result<Self, anyhow::Error> {
        let mut buf = [0u8; 64];
        five8::decode_64(s, &mut buf)?;
        ed25519_dalek::SigningKey::from_keypair_bytes(&buf)?;
        Ok(Keypair::from_bytes(&buf).unwrap())
    }
    fn new_adapter_wallet(pubkey: Pubkey) -> Self {
        let mut buf = [0u8; 64];
        buf[32..].copy_from_slice(&pubkey.to_bytes());
        Keypair::from_bytes(&buf).expect("correct size, never fail")
    }

    fn clone_keypair(&self) -> Self {
        Self::from_bytes(&self.to_bytes()).unwrap()
    }

    fn is_adapter_wallet(&self) -> bool {
        self.secret().as_bytes().iter().all(|b| *b == 0)
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default)]
struct AsAccountMetaImpl {
    #[serde_as(as = "AsPubkey")]
    pubkey: Pubkey,
    is_signer: bool,
    is_writable: bool,
}
fn account_meta_ser(i: &AccountMeta) -> AsAccountMetaImpl {
    AsAccountMetaImpl {
        pubkey: i.pubkey,
        is_signer: i.is_signer,
        is_writable: i.is_writable,
    }
}
fn account_meta_de(i: AsAccountMetaImpl) -> Result<AccountMeta, Infallible> {
    Ok(AccountMeta {
        pubkey: i.pubkey,
        is_signer: i.is_signer,
        is_writable: i.is_writable,
    })
}
serde_conv!(
    AsAccountMeta,
    AccountMeta,
    account_meta_ser,
    account_meta_de
);

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default)]
struct AsInstructionImpl {
    #[serde_as(as = "AsPubkey")]
    program_id: Pubkey,
    #[serde_as(as = "Vec<AsAccountMeta>")]
    accounts: Vec<AccountMeta>,
    #[serde_as(as = "serde_with::Bytes")]
    data: Vec<u8>,
}
fn instruction_ser(i: &Instruction) -> AsInstructionImpl {
    AsInstructionImpl {
        program_id: i.program_id,
        accounts: i.accounts.clone(),
        data: i.data.clone(),
    }
}
fn instruction_de(i: AsInstructionImpl) -> Result<Instruction, Infallible> {
    Ok(Instruction {
        program_id: i.program_id,
        accounts: i.accounts,
        data: i.data,
    })
}
serde_conv!(AsInstruction, Instruction, instruction_ser, instruction_de);

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Instructions {
    #[serde_as(as = "AsPubkey")]
    pub fee_payer: Pubkey,
    pub signers: Vec<Wallet>,
    #[serde_as(as = "Vec<AsInstruction>")]
    pub instructions: Vec<Instruction>,
}

fn is_set_compute_unit_price(
    message: &Message,
    index: usize,
    ins: &CompiledInstruction,
) -> Option<()> {
    let program_id = message.program_id(index)?;
    if compute_budget::check_id(program_id) {
        let data = ComputeBudgetInstruction::try_from_slice(&ins.data)
            .map_err(|error| tracing::error!("could not decode instruction: {}", error))
            .ok()?;
        matches!(data, ComputeBudgetInstruction::SetComputeUnitPrice(_)).then_some(())
    } else {
        None
    }
}

fn contains_set_compute_unit_price(message: &Message) -> bool {
    message
        .instructions
        .iter()
        .enumerate()
        .any(|(index, ins)| is_set_compute_unit_price(message, index, ins).is_some())
}

fn is_set_compute_unit_limit(
    message: &Message,
    index: usize,
    ins: &CompiledInstruction,
) -> Option<()> {
    let program_id = message.program_id(index)?;
    if compute_budget::check_id(program_id) {
        let data = ComputeBudgetInstruction::try_from_slice(&ins.data)
            .map_err(|error| tracing::error!("could not decode instruction: {}", error))
            .ok()?;
        matches!(data, ComputeBudgetInstruction::SetComputeUnitLimit(_)).then_some(())
    } else {
        None
    }
}

fn contains_set_compute_unit_limit(message: &Message) -> bool {
    message
        .instructions
        .iter()
        .enumerate()
        .any(|(index, ins)| is_set_compute_unit_limit(message, index, ins).is_some())
}

async fn get_priority_fee(message: &Message) -> Result<u64, anyhow::Error> {
    static HTTP: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);
    if let Ok(apikey) = std::env::var("HELIUS_API_KEY") {
        let helius = Helius::new(HTTP.clone(), &apikey);
        let network = SolanaNet::Mainnet;
        // TODO: not available on devnet and testnet
        // let network = SolanaNet::from_url(&rpc.url())
        //     .map_err(|_| tracing::warn!("could not guess cluster from url, using mainnet"))
        //     .unwrap_or(SolanaNet::Mainnet);
        let resp = helius
            .get_priority_fee_estimate(
                network.as_str(),
                GetPriorityFeeEstimateRequest {
                    account_keys: Some(
                        message
                            .account_keys
                            .iter()
                            .map(|pk| pk.to_string())
                            .collect(),
                    ),
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

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub enum InsertionBehavior {
    #[default]
    Auto,
    No,
    Value(u64),
}

impl FromStr for InsertionBehavior {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "auto" => InsertionBehavior::Auto,
            "no" => InsertionBehavior::No,
            s => InsertionBehavior::Value(s.parse()?),
        })
    }
}

impl Display for InsertionBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertionBehavior::Auto => f.write_str("auto"),
            InsertionBehavior::No => f.write_str("no"),
            InsertionBehavior::Value(v) => v.fmt(f),
        }
    }
}

impl Serialize for InsertionBehavior {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for InsertionBehavior {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        <Cow<'de, str> as Deserialize>::deserialize(deserializer)?
            .parse()
            .map_err(D::Error::custom)
    }
}

const fn default_simulation_level() -> CommitmentLevel {
    CommitmentLevel::Finalized
}

const fn default_tx_level() -> CommitmentLevel {
    CommitmentLevel::Confirmed
}

const fn default_wait_level() -> CommitmentLevel {
    CommitmentLevel::Confirmed
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum KeypairOrPubkey {
    Keypair(Wallet),
    Pubkey(#[serde_as(as = "AsPubkey")] Pubkey),
}

impl KeypairOrPubkey {
    pub fn to_keypair(self) -> Wallet {
        match self {
            KeypairOrPubkey::Keypair(k) => k,
            KeypairOrPubkey::Pubkey(public_key) => Wallet::Adapter { public_key },
        }
    }
}

#[serde_with::serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct ExecutionConfig {
    pub overwrite_feepayer: Option<KeypairOrPubkey>,

    #[serde(default)]
    pub compute_budget: InsertionBehavior,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub fallback_compute_budget: Option<u64>,
    #[serde(default)]
    pub priority_fee: InsertionBehavior,

    #[serde(default = "default_simulation_level")]
    pub simulation_commitment_level: CommitmentLevel,
    #[serde(default = "default_tx_level")]
    pub tx_commitment_level: CommitmentLevel,
    #[serde(default = "default_wait_level")]
    pub wait_commitment_level: CommitmentLevel,

    #[serde(skip)]
    pub execute_on: ExecuteOn,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SolanaActionConfig {
    #[serde(with = "value::pubkey")]
    pub action_signer: Pubkey,
    #[serde(with = "value::pubkey")]
    pub action_identity: Pubkey,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub enum ExecuteOn {
    SolanaAction(SolanaActionConfig),
    #[default]
    CurrentMachine,
}

impl ExecutionConfig {
    pub fn from_env(map: &HashMap<String, String>) -> Result<Self, value::Error> {
        let map = map
            .iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect::<value::Map>();
        value::from_map(map)
    }
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            overwrite_feepayer: None,
            compute_budget: InsertionBehavior::default(),
            fallback_compute_budget: None,
            priority_fee: InsertionBehavior::default(),
            simulation_commitment_level: default_simulation_level(),
            tx_commitment_level: default_tx_level(),
            wait_commitment_level: default_wait_level(),
            execute_on: ExecuteOn::default(),
        }
    }
}

fn commitment(commitment: CommitmentLevel) -> CommitmentConfig {
    CommitmentConfig { commitment }
}

pub fn build_action_reference(timestamp: i64, run_id: FlowRunId) -> Vec<u8> {
    let reference_bytes = [
        timestamp.to_le_bytes().as_ref(),
        run_id.into_bytes().as_ref(),
        &[0u8; 8], // unused
    ]
    .concat();
    debug_assert_eq!(reference_bytes.len(), 32);
    reference_bytes
}

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

async fn action_identity_memo(
    identity: Pubkey,
    run_id: FlowRunId,
    signer: &mut signer::Svc,
) -> Result<String, signer::Error> {
    let reference_bytes = build_action_reference(Utc::now().timestamp(), run_id);
    let reference = bs58::encode(&reference_bytes).into_string();
    let signature = signer
        .call_mut(signer::SignatureRequest {
            id: None,
            time: Utc::now(),
            pubkey: identity,
            message: reference_bytes.into(),
            timeout: SIGNATURE_TIMEOUT,
            flow_run_id: Some(run_id),
            signatures: None,
        })
        .await?
        .signature;
    Ok(format!(
        "solana-action:{}:{}:{}",
        identity, reference, signature
    ))

    // Ok(Instruction {
    //     program_id: spl_memo::v1::ID,
    //     accounts: Vec::new(),
    //     data: memo.as_bytes().to_owned(),
    // })
    // memo v3 fails
    // Ok(spl_memo::build_memo(memo.as_bytes(), signers))
}

impl Instructions {
    fn push_signer(&mut self, new: Wallet) {
        let old = self.signers.iter_mut().find(|k| k.pubkey() == new.pubkey());
        if let Some(old) = old {
            if old.is_adapter_wallet() {
                // prefer hardcoded
                *old = new;
            }
        } else {
            self.signers.push(new);
        }
    }

    pub fn set_feepayer(&mut self, signer: Wallet) {
        self.fee_payer = signer.pubkey();
        self.push_signer(signer);
    }

    pub fn combine(&mut self, next: Self) -> Result<(), Self> {
        if next.fee_payer != self.fee_payer {
            return Err(next);
        }

        for new in next.signers {
            self.push_signer(new);
        }

        self.instructions.extend(next.instructions);

        Ok(())
    }

    async fn insert_priority_fee(
        &mut self,
        rpc: &RpcClient,
        config: &ExecutionConfig,
    ) -> Result<usize, Error> {
        let message = Message::new_with_blockhash(
            &self.instructions,
            Some(&self.fee_payer),
            &rpc.get_latest_blockhash_with_commitment(commitment(
                config.simulation_commitment_level,
            ))
            .await
            .map_err(|error| Error::solana(error, 0))?
            .0,
        );
        let count = self.instructions.len();

        let mut inserted = 0;

        if config.compute_budget != InsertionBehavior::No
            && !contains_set_compute_unit_limit(&message)
        {
            let compute_units = if let InsertionBehavior::Value(x) = config.compute_budget {
                x
            } else {
                match rpc
                    .simulate_transaction(&Transaction::new_unsigned(message.clone()))
                    .await
                {
                    Err(error) => {
                        tracing::warn!("simulation failed: {}", error);
                        None
                    }
                    Ok(result) => {
                        if let Some(error) = result.value.err {
                            tracing::warn!("simulation error: {}", error);
                            for log in result.value.logs.unwrap_or_default() {
                                tracing::info!("{}", log);
                            }
                        } else {
                            for log in result.value.logs.unwrap_or_default() {
                                tracing::debug!("{}", log);
                            }
                        }
                        let consumed = result.value.units_consumed;
                        if consumed.is_none() || consumed == Some(0) {
                            None
                        } else {
                            consumed.map(|x| 1000 + x * 3 / 2)
                        }
                    }
                }
                .or(config.fallback_compute_budget)
                .unwrap_or(200_000 * count as u64)
            }
            .min(1_400_000) as u32;
            tracing::info!("setting compute unit limit {}", compute_units);
            self.instructions.insert(
                0,
                ComputeBudgetInstruction::set_compute_unit_limit(compute_units),
            );
            inserted += 1;
        }

        if config.priority_fee != InsertionBehavior::No
            && !contains_set_compute_unit_price(&message)
        {
            let fee = if let InsertionBehavior::Value(x) = config.priority_fee {
                x
            } else {
                get_priority_fee(&message)
                    .await
                    .map_err(|error| {
                        tracing::warn!("get_priority_fee error: {}", error);
                    })
                    .unwrap_or(100)
            };
            tracing::info!("adding priority fee {}", fee);
            self.instructions
                .insert(0, ComputeBudgetInstruction::set_compute_unit_price(fee));
            inserted += 1;
        }

        Ok(inserted)
    }

    async fn build_message(
        &mut self,
        rpc: &RpcClient,
        config: &ExecutionConfig,
    ) -> Result<(Message, usize), Error> {
        let inserted = self.insert_priority_fee(rpc, config).await?;

        let message = Message::new_with_blockhash(
            &self.instructions,
            Some(&self.fee_payer),
            &rpc.get_latest_blockhash_with_commitment(commitment(config.tx_commitment_level))
                .await
                .map_err(|error| Error::solana(error, inserted))?
                .0,
        );

        Ok((message, inserted))
    }

    pub async fn build_for_solana_action(
        mut self,
        action_signer: Pubkey,
        action_identity: Option<Pubkey>,
        rpc: &RpcClient,
        mut signer: signer::Svc,
        flow_run_id: Option<FlowRunId>,
        config: &ExecutionConfig,
    ) -> Result<(Transaction, usize, Option<String>), Error> {
        let inserted = self.insert_priority_fee(rpc, config).await?;
        let memo = if let Some(action_identity) = action_identity {
            if !self
                .instructions
                .iter()
                .flat_map(|i| i.accounts.iter())
                .any(|a| a.pubkey == action_identity)
            {
                let non_memo = self
                    .instructions
                    .iter_mut()
                    .find(|i| i.program_id != spl_memo::ID && i.program_id != spl_memo::v1::ID)
                    .ok_or_else(|| Error::other("at least 1 non-memo instruction is required"))?;
                non_memo.accounts.push(AccountMeta {
                    pubkey: action_identity,
                    is_signer: false,
                    is_writable: false,
                });
            }
            let memo = action_identity_memo(
                action_identity,
                flow_run_id.unwrap_or_default(),
                &mut signer,
            )
            .await
            .map_err(Error::other)?;
            self.instructions.push(Instruction {
                program_id: spl_memo::v1::ID,
                accounts: Vec::new(),
                data: memo.as_bytes().to_owned(),
            });
            Some(memo)
        } else {
            None
        };

        let message = Message::new_with_blockhash(
            &self.instructions,
            Some(&self.fee_payer),
            &rpc.get_latest_blockhash_with_commitment(commitment(config.tx_commitment_level))
                .await
                .map_err(|error| Error::solana(error, inserted))?
                .0,
        );

        // Sign all signatures except for action_signer
        let wallets = self
            .signers
            .iter()
            .filter(|keypair| keypair.is_adapter_wallet() && keypair.pubkey() != action_signer)
            .map(|keypair| keypair.pubkey())
            .collect::<BTreeSet<_>>();
        let data: Bytes = message.serialize().into();
        let reqs = wallets
            .iter()
            .map(|&pubkey| signer::SignatureRequest {
                id: None,
                time: Utc::now(),
                pubkey,
                message: data.clone(),
                timeout: SIGNATURE_TIMEOUT,
                flow_run_id,
                signatures: None,
            })
            .collect::<Vec<_>>();

        let signature_results = tokio::time::timeout(
            SIGNATURE_TIMEOUT,
            signer
                .call_all(futures::stream::iter(reqs))
                .try_collect::<Vec<_>>(),
        )
        .await
        .map_err(|_| Error::Timeout)??;

        let presigners = wallets
            .iter()
            .zip(signature_results.iter())
            .map(|(pk, resp)| {
                (resp.new_message.is_none() || *resp.new_message.as_ref().unwrap() == data)
                    .then(|| Presigner::new(pk, &resp.signature))
                    .ok_or_else(|| {
                        anyhow!("{} signature failed: not allowed to change transaction", pk)
                    })
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()?;

        let tx = {
            let mut signers = Vec::<&dyn Signer>::with_capacity(self.signers.len() - 1);

            for p in &presigners {
                signers.push(p);
            }

            for k in self.signers.iter().filter_map(|w| w.keypair()) {
                if k.pubkey() != action_signer {
                    signers.push(k);
                }
            }

            let blockhash = message.recent_blockhash;
            let mut tx = Transaction::new_unsigned(message);
            tx.try_partial_sign(&signers, blockhash)?;
            tx
        };

        Ok((tx, inserted, memo))
    }

    async fn build_and_sign_tx(
        mut self,
        rpc: &RpcClient,
        signer: signer::Svc,
        flow_run_id: Option<FlowRunId>,
        config: &ExecutionConfig,
    ) -> Result<(Transaction, usize), Error> {
        let (mut message, inserted) = self.build_message(rpc, config).await?;
        let mut data: Bytes = message.serialize().into();
        let fee_payer_signature = {
            let keypair = self
                .signers
                .iter()
                .find(|w| w.pubkey() == self.fee_payer)
                .ok_or_else(|| Error::other("fee payer is not in signers"))?;

            tracing::info!("{} signing", keypair.pubkey());
            if let Some(keypair) = keypair.keypair() {
                keypair.sign_message(&data)
            } else {
                let fut = signer.call_ref(signer::SignatureRequest {
                    id: None,
                    time: Utc::now(),
                    pubkey: keypair.pubkey(),
                    message: data.clone(),
                    timeout: SIGNATURE_TIMEOUT,
                    flow_run_id,
                    signatures: None,
                });
                let resp = tokio::time::timeout(SIGNATURE_TIMEOUT, fut)
                    .await
                    .map_err(|_| Error::Timeout)?
                    .map_err(Error::other)?;
                if let Some(new) = resp.new_message {
                    let new_message = is_same_message_logic(&data, &new)?;
                    tracing::info!("updating transaction");
                    message = new_message;
                    data = new;
                }
                resp.signature
            }
        };

        let wallets = self
            .signers
            .iter()
            .filter_map(|k| {
                if k.is_adapter_wallet() && k.pubkey() != self.fee_payer {
                    Some(k.pubkey())
                } else {
                    None
                }
            })
            .collect::<BTreeSet<_>>();

        let reqs = wallets
            .iter()
            .map(|&pubkey| signer::SignatureRequest {
                id: None,
                time: Utc::now(),
                pubkey,
                message: data.clone(),
                timeout: SIGNATURE_TIMEOUT,
                flow_run_id,
                signatures: Some(
                    [signer::Presigner {
                        pubkey: self.fee_payer,
                        signature: fee_payer_signature,
                    }]
                    .into(),
                ),
            })
            .collect::<Vec<_>>();

        let signature_results = tokio::time::timeout(
            SIGNATURE_TIMEOUT,
            signer
                .call_all(futures::stream::iter(reqs))
                .try_collect::<Vec<_>>(),
        )
        .await
        .map_err(|_| Error::Timeout)??;

        let tx = {
            let mut presigners = wallets
                .iter()
                .zip(signature_results.iter())
                .map(|(pk, resp)| {
                    (resp.new_message.is_none() || *resp.new_message.as_ref().unwrap() == data)
                        .then(|| Presigner::new(pk, &resp.signature))
                        .ok_or_else(|| {
                            anyhow!("{} signature failed: not allowed to change transaction", pk)
                        })
                })
                .collect::<Result<Vec<_>, anyhow::Error>>()?;
            presigners.push(Presigner::new(&self.fee_payer, &fee_payer_signature));

            let mut signers = Vec::<&dyn Signer>::with_capacity(self.signers.len());

            for p in &presigners {
                signers.push(p);
            }

            for k in self.signers.iter().filter_map(|w| w.keypair()) {
                if k.pubkey() != self.fee_payer {
                    signers.push(k);
                }
            }

            let blockhash = message.recent_blockhash;
            let mut tx = Transaction::new_unsigned(message);
            tx.try_sign(&signers, blockhash)?;

            tx
        };

        Ok((tx, inserted))
    }

    async fn execute_current_machine(
        self,
        rpc: &RpcClient,
        signer: signer::Svc,
        flow_run_id: Option<FlowRunId>,
        config: &ExecutionConfig,
    ) -> Result<Signature, Error> {
        let (tx, inserted) = self
            .build_and_sign_tx(rpc, signer, flow_run_id, config)
            .await?;

        // SDK's error message of verify_precompiles is not infomative,
        // so we run it manually
        // TODO: is it correct to use FeatureSet::all_enabled()?
        verify_precompiles(&tx, &FeatureSet::all_enabled())?;

        let signature = rpc
            .send_transaction_with_config(
                &tx,
                RpcSendTransactionConfig {
                    preflight_commitment: Some(config.tx_commitment_level),
                    ..<_>::default()
                },
            )
            .await
            .map_err(move |error| Error::solana(error, inserted))?;
        tracing::info!("submitted {}", signature);

        confirm_transaction(
            rpc,
            &signature,
            tx.get_recent_blockhash(),
            commitment(config.wait_commitment_level),
        )
        .await
        .map_err(move |error| Error::solana(error, inserted))?;

        Ok(signature)
    }

    async fn execute_solana_action(
        self,
        rpc: &RpcClient,
        mut signer: signer::Svc,
        flow_run_id: Option<FlowRunId>,
        config: &ExecutionConfig,
        action_config: &SolanaActionConfig,
    ) -> Result<Signature, Error> {
        let (tx, _, memo) = self
            .build_for_solana_action(
                action_config.action_signer,
                Some(action_config.action_identity),
                rpc,
                signer.clone(),
                flow_run_id,
                config,
            )
            .await?;
        let memo = memo.expect("action_identity != None");
        let req = signer::SignatureRequest {
            id: None,
            time: Utc::now(),
            pubkey: action_config.action_signer,
            message: tx.message_data().into(),
            timeout: Duration::from_secs(0),
            flow_run_id,
            signatures: list_signatures(&tx),
        };
        if let Err(error) = signer.call_mut(req).await {
            if !matches!(error, signer::Error::Timeout) {
                return Err(Error::other(error));
            }
        }
        let signature = confirm_action_transaction(
            rpc,
            action_config.action_identity,
            memo,
            config.wait_commitment_level,
        )
        .await?;
        Ok(signature)
    }

    pub async fn execute(
        self,
        rpc: &RpcClient,
        signer: signer::Svc,
        flow_run_id: Option<FlowRunId>,
        config: ExecutionConfig,
    ) -> Result<Signature, Error> {
        match &config.execute_on {
            ExecuteOn::CurrentMachine => {
                self.execute_current_machine(rpc, signer, flow_run_id, &config)
                    .await
            }
            ExecuteOn::SolanaAction(action_config) => {
                self.execute_solana_action(rpc, signer, flow_run_id, &config, action_config)
                    .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::env::{
        COMPUTE_BUDGET, FALLBACK_COMPUTE_BUDGET, OVERWRITE_FEEPAYER, PRIORITY_FEE,
        SIMULATION_COMMITMENT_LEVEL, TX_COMMITMENT_LEVEL, WAIT_COMMITMENT_LEVEL,
    };
    use base64::prelude::*;
    use solana_sdk::{pubkey, system_instruction::transfer};

    #[test]
    fn test_wallet_serde() {
        let keypair = Keypair::new();
        let input = Value::String(keypair.to_base58_string());
        let Wallet::Keypair(result) = value::from_value(input).unwrap() else {
            panic!()
        };
        assert_eq!(result.to_base58_string(), keypair.to_base58_string());
    }

    #[test]
    fn test_compare_msg_logic() {
        const OLD: &str = "AwEJE/I9QMIByO+GhMkfll9MXSsAYs1ITPmKAfxGS/USlNwuw0EUt8a41tLSp95YmtHPKWDGGcApBC0AEmN1Sd+5kfDOAq0G+/qWg2KKmXfDQF1HIuw9Op9LiSZK5iA7jcVQ9wceNyYLLzZIZ+cVomhs1zT04hQeIKdXkiMyUpH9KA95JukMx1A93RFsivUbXmW+wwO52yE0+21NxUpXL/eMTCpS1wQ6IUwmvO0o13hn6qE0Pi73WxtEGjlbBilP+HVyqFkAIKLtjJBJ25Jae9iO3Xe17TFanfbTgtEbgKAJ5nWVuJt84ctKVWEXbuPgqHbe6H8fchmNtE0iKLjuVOE0AJ3GIRyraKaGg0wqZXXkbS0qr6CQYxZVv7PeO7zsL/swgPucBbMHhqVF+Mv8NimuycfvB72jxeN3uhwn+c715MdKAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADBkZv5SEXMv/srbpyw5vnvIzlu8X3EmssQ5s6QAAAAAan1RcYe9FmNdrUBFX9wsDBJMaPIVZ1pdu6y18IAAAABt324ddloZPZy+FGzut5rBy0he1fWzeROoz1hX7/AKkLcGWx49F8RTidUn9rBMPNWLhscxqg/bVJttG8A/gpRlM2SFRbPsgTT3LuOBLPsJzpVN5CeDaecGGyxbawEE6Kcy72NeMo2v4ccHESWqcHq3GioOBRqLHY25fQEpaeCVSLCKI3/q1QflOctOQHXPk3VuQhThJQPfn/dD3sEZbonYyXJY9OJInxuz0QKRSODYMLWhOZ2v8QhASOe9jb6fhZdtEfrjiMo8c/EYJzRiXnOLehdv4i42eBpdbr4NYTAzkICwAJA+gDAAAAAAAACwAFAkANAwAOCQMFAQIAAgoMDdoBKgAYAAAAU3BhY2UgT3BlcmF0b3IgQ2hhbWVsZW9uBAAAAFNQT0NTAAAAaHR0cHM6Ly9hc3NldHMuc3BhY2VvcGVyYXRvci5jb20vbWV0YWRhdGEvMzU4NjY4MzItN2M4My00OWM2LWJmZjctY2FhMDBiNmE2NDE1Lmpzb276AAEBAAAAzgKtBvv6loNiipl3w0BdRyLsPTqfS4kmSuYgO43FUPcAZAABBAEAiwiiN/6tUH5TnLTkB1z5N1bkIU4SUD35/3Q97BGW6J0AAAABAAEBZAAAAAAAAAAOCAIOAxEJDwoMAjQBDggCDgMODg4KDAI0AA4OBxADBQQBCAIACgwNDg4DLAMADg8IAAMFBAECDgAKDA0SDg4LKwABAAAAAAAAAAAKAgAGDAIAAAAAu+6gAAAAAA==";
        const NEW: &str = "AwEJE/I9QMIByO+GhMkfll9MXSsAYs1ITPmKAfxGS/USlNwuw0EUt8a41tLSp95YmtHPKWDGGcApBC0AEmN1Sd+5kfDOAq0G+/qWg2KKmXfDQF1HIuw9Op9LiSZK5iA7jcVQ9ybpDMdQPd0RbIr1G15lvsMDudshNPttTcVKVy/3jEwqUtcEOiFMJrztKNd4Z+qhND4u91sbRBo5WwYpT/h1cqhZACCi7YyQSduSWnvYjt13te0xWp3204LRG4CgCeZ1lbibfOHLSlVhF27j4Kh23uh/H3IZjbRNIii47lThNACdxiEcq2imhoNMKmV15G0tKq+gkGMWVb+z3ju87C/7MID7nAWzB4alRfjL/DYprsnH7we9o8Xjd7ocJ/nO9eTHSgceNyYLLzZIZ+cVomhs1zT04hQeIKdXkiMyUpH9KA95AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABTNkhUWz7IE09y7jgSz7Cc6VTeQng2nnBhssW2sBBOinMu9jXjKNr+HHBxElqnB6txoqDgUaix2NuX0BKWnglUiwiiN/6tUH5TnLTkB1z5N1bkIU4SUD35/3Q97BGW6J2MlyWPTiSJ8bs9ECkUjg2DC1oTmdr/EIQEjnvY2+n4WQMGRm/lIRcy/+ytunLDm+e8jOW7xfcSayxDmzpAAAAAC3BlsePRfEU4nVJ/awTDzVi4bHMaoP21SbbRvAP4KUYGp9UXGHvRZjXa1ARV/cLAwSTGjyFWdaXbustfCAAAAAbd9uHXZaGT2cvhRs7reawctIXtX1s3kTqM9YV+/wCpdtEfrjiMo8c/EYJzRiXnOLehdv4i42eBpdbr4NYTAzkIDwAJA+gDAAAAAAAADwAFAkANAwAQCQkEAQIAAgoREtoBKgAYAAAAU3BhY2UgT3BlcmF0b3IgQ2hhbWVsZW9uBAAAAFNQT0NTAAAAaHR0cHM6Ly9hc3NldHMuc3BhY2VvcGVyYXRvci5jb20vbWV0YWRhdGEvMzU4NjY4MzItN2M4My00OWM2LWJmZjctY2FhMDBiNmE2NDE1Lmpzb276AAEBAAAAzgKtBvv6loNiipl3w0BdRyLsPTqfS4kmSuYgO43FUPcAZAABBAEAiwiiN/6tUH5TnLTkB1z5N1bkIU4SUD35/3Q97BGW6J0AAAABAAEBZAAAAAAAAAAQCAIQCQ0ICwoRAjQBEAgCEAkQEBAKEQI0ABAOBgwJBAMBBwIAChESEBADLAMAEA8HAAkEAwECEAAKERIOEBALKwABAAAAAAAAAAAKAgAFDAIAAAAAu+6gAAAAAA==";
        is_same_message_logic(
            &BASE64_STANDARD.decode(OLD).unwrap(),
            &BASE64_STANDARD.decode(NEW).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_parse_config() {
        fn t<const N: usize>(kv: [(&str, &str); N], result: ExecutionConfig) {
            let map = kv
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect::<HashMap<_, _>>();
            let c = ExecutionConfig::from_env(&map).unwrap();
            let l = serde_json::to_string_pretty(&c).unwrap();
            let r = serde_json::to_string_pretty(&result).unwrap();
            assert_eq!(l, r);
        }
        t(
            [(
                OVERWRITE_FEEPAYER,
                "HJbqSuV94woJfyxFNnJyfQdACvvJYaNWsW1x6wmJ8kiq",
            )],
            ExecutionConfig {
                overwrite_feepayer: Some(KeypairOrPubkey::Pubkey(pubkey!(
                    "HJbqSuV94woJfyxFNnJyfQdACvvJYaNWsW1x6wmJ8kiq"
                ))),
                ..<_>::default()
            },
        );
        t(
            [
                (COMPUTE_BUDGET, "auto"),
                (FALLBACK_COMPUTE_BUDGET, "500000"),
                (PRIORITY_FEE, "1000"),
                (SIMULATION_COMMITMENT_LEVEL, "confirmed"),
                (TX_COMMITMENT_LEVEL, "finalized"),
                (WAIT_COMMITMENT_LEVEL, "processed"),
            ],
            ExecutionConfig {
                compute_budget: InsertionBehavior::Auto,
                fallback_compute_budget: Some(500000),
                priority_fee: InsertionBehavior::Value(1000),
                simulation_commitment_level: CommitmentLevel::Confirmed,
                tx_commitment_level: CommitmentLevel::Finalized,
                wait_commitment_level: CommitmentLevel::Processed,
                ..<_>::default()
            },
        );
    }

    #[tokio::test]
    async fn test_build_message() {
        let from = Keypair::new();
        let to = Pubkey::new_unique();

        let rpc = RpcClient::new(SolanaNet::Devnet.url().to_owned());
        let mut ins = Instructions {
            fee_payer: from.pubkey(),
            signers: [from.clone_keypair().into()].into(),
            instructions: [transfer(&from.pubkey(), &to, 100000)].into(),
        };
        let (_msg, inserted) = ins.build_message(&rpc, &<_>::default()).await.unwrap();
        assert_eq!(inserted, 2);

        let mut ins = Instructions {
            fee_payer: from.pubkey(),
            signers: [from.clone_keypair().into()].into(),
            instructions: [transfer(&from.pubkey(), &to, 100000)].into(),
        };
        let (_msg, inserted) = ins
            .build_message(
                &rpc,
                &ExecutionConfig {
                    priority_fee: InsertionBehavior::No,
                    ..<_>::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(inserted, 1);
    }

    #[test]
    fn test_keypair_or_pubkey_keypair() {
        let keypair = Keypair::new();
        let x = KeypairOrPubkey::Keypair(Wallet::Keypair(keypair.clone_keypair()));
        let value = value::to_value(&x).unwrap();
        assert_eq!(value, Value::B64(keypair.to_bytes()));
        assert_eq!(value::from_value::<KeypairOrPubkey>(value).unwrap(), x);
    }

    #[test]
    fn test_keypair_or_pubkey_adapter() {
        let pubkey = Pubkey::new_unique();
        let x = KeypairOrPubkey::Keypair(Wallet::Adapter {
            public_key: pubkey.clone(),
        });
        let value = value::to_value(&x).unwrap();
        assert_eq!(
            value,
            Value::Map(value::map! {
                "public_key" => pubkey,
            })
        );
        assert_eq!(value::from_value::<KeypairOrPubkey>(value).unwrap(), x);
    }

    #[test]
    fn test_keypair_or_pubkey_pubkey() {
        let pubkey = Pubkey::new_unique();
        let x = KeypairOrPubkey::Pubkey(pubkey.clone());
        let value = value::to_value(&x).unwrap();
        assert_eq!(value, Value::B32(pubkey.to_bytes()));
        assert_eq!(value::from_value::<KeypairOrPubkey>(value).unwrap(), x);
    }

    #[test]
    fn test_wallet_keypair() {
        let keypair = Keypair::new();
        let x = Wallet::Keypair(keypair.clone_keypair());
        let value = value::to_value(&x).unwrap();
        assert_eq!(value, Value::B64(keypair.to_bytes()));
        assert_eq!(value::from_value::<Wallet>(value).unwrap(), x);
    }

    #[test]
    fn test_wallet_adapter() {
        let pubkey = Pubkey::new_unique();
        let x = Wallet::Adapter {
            public_key: pubkey.clone(),
        };
        let value = value::to_value(&x).unwrap();
        assert_eq!(
            value,
            Value::Map(value::map! {
                "public_key" => pubkey,
            })
        );
        assert_eq!(value::from_value::<Wallet>(value).unwrap(), x);
    }
}
