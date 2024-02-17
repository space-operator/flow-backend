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
use solana_client::{
    client_error::{ClientError, ClientErrorKind},
    nonblocking::rpc_client::RpcClient,
    rpc_request::{RpcError, RpcResponseErrorData},
    rpc_response::RpcSimulateTransactionResult,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::{self, ComputeBudgetInstruction},
    feature_set::FeatureSet,
    instruction::{CompiledInstruction, Instruction},
    message::Message,
    precompiles::verify_if_precompile,
    sanitize::Sanitize,
    signature::Presigner,
    signer::{keypair::Keypair, Signer},
    transaction::Transaction,
};
use spo_helius::{
    GetPriorityFeeEstimateOptions, GetPriorityFeeEstimateRequest, Helius, PriorityLevel,
};
use std::time::Duration;
use tower::ServiceExt;

pub const SIGNATURE_TIMEOUT: Duration = Duration::from_secs(3 * 60);

pub use solana_sdk::pubkey::Pubkey;
pub use solana_sdk::signature::Signature;

/// `l` is old, `r` is new
pub fn is_same_message_logic(l: &[u8], r: &[u8]) -> Result<Message, anyhow::Error> {
    let l = bincode::deserialize::<Message>(&l)?;
    let r = bincode::deserialize::<Message>(&r)?;
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

pub trait KeypairExt {
    fn new_adapter_wallet(pk: Pubkey) -> Self;
    fn clone_keypair(&self) -> Self;
    fn is_adapter_wallet(&self) -> bool;
}

impl KeypairExt for Keypair {
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

#[derive(Default, Debug)]
pub struct Instructions {
    pub fee_payer: Pubkey,
    pub signers: Vec<Keypair>,
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

async fn get_priority_fee(message: &Message, _rpc: &RpcClient) -> Result<u64, anyhow::Error> {
    static HTTP: Lazy<reqwest::Client> = Lazy::new(|| reqwest::Client::new());
    if let Some(apikey) = std::env::var("HELIUS_API_KEY").ok() {
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

impl Instructions {
    fn push_signer(&mut self, new: Keypair) {
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

    pub fn set_feepayer(&mut self, signer: Keypair) {
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

    pub async fn execute(
        mut self,
        rpc: &RpcClient,
        signer: signer::Svc,
        flow_run_id: Option<FlowRunId>,
    ) -> Result<Signature, Error> {
        let mut message = Message::new_with_blockhash(
            &self.instructions,
            Some(&self.fee_payer),
            &rpc.get_latest_blockhash()
                .await
                .map_err(|error| Error::solana(error, 0))?,
        );

        let count = self.instructions.len();
        let compute_units = match rpc
            .simulate_transaction(&Transaction::new_unsigned(message.clone()))
            .await
        {
            Err(error) => {
                tracing::warn!("simulation failed: {}", error);
                None
            }
            Ok(result) => result.value.units_consumed.map(|x| x * 3 / 2),
        }
        .unwrap_or_else(|| 100000 * count as u64)
        .min(1400000) as u32;

        let inserted = if !contains_set_compute_unit_price(&message) {
            match get_priority_fee(&message, rpc).await {
                Ok(fee) => {
                    tracing::info!("setting compute unit limit {}", compute_units);
                    self.instructions.insert(
                        0,
                        ComputeBudgetInstruction::set_compute_unit_limit(compute_units),
                    );
                    tracing::info!("adding priority fee {}", fee);
                    self.instructions
                        .insert(0, ComputeBudgetInstruction::set_compute_unit_price(fee));
                    message = Message::new_with_blockhash(
                        &self.instructions,
                        Some(&self.fee_payer),
                        &message.recent_blockhash,
                    );
                    2
                }
                Err(error) => {
                    tracing::warn!("{}, skipping priority fee", error);
                    0
                }
            }
        } else {
            0
        };

        message.recent_blockhash = rpc
            .get_latest_blockhash()
            .await
            .map_err(|error| Error::solana(error, inserted))?;

        let mut data: Bytes = message.serialize().into();

        tracing::info!("executing transaction");
        tracing::info!("message size: {}", data.len());
        tracing::info!("fee payer: {}", self.fee_payer);

        let fee_payer_signature = {
            let keypair = self
                .signers
                .iter()
                .find(|w| w.pubkey() == self.fee_payer)
                .ok_or_else(|| Error::other("fee payer is not in signers"))?;

            tracing::info!("{} signing", keypair.pubkey());
            if keypair.is_adapter_wallet() {
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
                    .map_err(|error| Error::other(error))?;
                if let Some(new) = resp.new_message {
                    let new_message = is_same_message_logic(&data, &new)?;
                    tracing::info!("updating transaction");
                    message = new_message;
                    data = new;
                }
                resp.signature
            } else {
                keypair.sign_message(&data)
            }
        };

        let mut wallets = self
            .signers
            .iter()
            .filter_map(|k| {
                if k.is_adapter_wallet() && k.pubkey() != self.fee_payer {
                    Some(k.pubkey())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        wallets.sort();
        wallets.dedup();

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

        let fut = signer
            .call_all(futures::stream::iter(reqs))
            .try_collect::<Vec<_>>();

        let sigs = tokio::time::timeout(SIGNATURE_TIMEOUT, fut)
            .await
            .map_err(|_| Error::Timeout)??;

        let tx = {
            let mut presigners = wallets
                .iter()
                .zip(sigs.iter())
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

            for k in &self.signers {
                if !k.is_adapter_wallet() && k.pubkey() != self.fee_payer {
                    signers.push(k);
                }
            }

            let mut tx = Transaction::new_unsigned(message);
            tx.try_sign(&signers, tx.message.recent_blockhash)?;
            tx
        };

        // TODO: is it correct to use FeatureSet::all_enabled()?
        verify_precompiles(&tx, &FeatureSet::all_enabled())?;

        let commitment = CommitmentConfig::confirmed();
        let sig = rpc
            .send_and_confirm_transaction_with_spinner_and_commitment(&tx, commitment)
            .await
            .map_err(move |error| Error::solana(error, inserted))?;

        Ok(sig)
    }
}

/// Verify the precompiled programs in this transaction.
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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::prelude::*;
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
}
