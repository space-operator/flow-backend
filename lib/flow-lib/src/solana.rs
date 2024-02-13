use crate::{
    context::{execute::Error, signer},
    FlowRunId, SolanaNet,
};
use anyhow::{anyhow, bail};
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
    compute_budget::ComputeBudgetInstruction,
    feature_set::FeatureSet,
    instruction::{CompiledInstruction, Instruction},
    message::Message,
    precompiles::verify_if_precompile,
    signature::Presigner,
    signer::{keypair::Keypair, Signer},
    transaction::Transaction,
};
use spo_helius::{
    GetPriorityFeeEstimateOptions, GetPriorityFeeEstimateRequest, Helius, PriorityLevel,
};
use std::time::Duration;
use tower::ServiceExt;

pub const SIGNATURE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

pub use solana_sdk::pubkey::Pubkey;
pub use solana_sdk::signature::Signature;

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
    tx: &Transaction,
    index: usize,
    ins: &CompiledInstruction,
) -> Option<()> {
    let program_id = tx.message.program_id(index)?;
    if solana_sdk::compute_budget::check_id(program_id) {
        let data = ComputeBudgetInstruction::try_from_slice(&ins.data)
            .map_err(|error| tracing::error!("could not decode instruction: {}", error))
            .ok()?;
        matches!(data, ComputeBudgetInstruction::SetComputeUnitPrice(_)).then_some(())
    } else {
        None
    }
}

fn contains_set_compute_unit_price(tx: &Transaction) -> bool {
    tx.message
        .instructions
        .iter()
        .enumerate()
        .any(|(index, ins)| is_set_compute_unit_price(tx, index, ins).is_some())
}

async fn get_priority_fee(tx: &Transaction, rpc: &RpcClient) -> Result<u64, anyhow::Error> {
    static HTTP: Lazy<reqwest::Client> = Lazy::new(|| reqwest::Client::new());
    if let Some(apikey) = std::env::var("HELIUS_API_KEY").ok() {
        let helius = Helius::new(HTTP.clone(), &apikey);
        let network = SolanaNet::from_url(&rpc.url())
            .map_err(|_| tracing::warn!("could not guess cluster from url, using mainnet"))
            .unwrap_or(SolanaNet::Mainnet);
        let resp = helius
            .get_priority_fee_estimate(
                network.as_str(),
                GetPriorityFeeEstimateRequest {
                    account_keys: Some(
                        tx.message
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
        let message = Message::new_with_blockhash(
            &self.instructions,
            Some(&self.fee_payer),
            &rpc.get_latest_blockhash().await?,
        );

        let mut tx = Transaction::new_unsigned(message);

        if !contains_set_compute_unit_price(&tx) {
            match get_priority_fee(&tx, rpc).await {
                Ok(fee) => {
                    tracing::info!("adding priority fee {}", fee);
                    self.instructions
                        .push(ComputeBudgetInstruction::set_compute_unit_limit(200000));
                    self.instructions
                        .push(ComputeBudgetInstruction::set_compute_unit_price(fee));
                    let message = Message::new_with_blockhash(
                        &self.instructions,
                        Some(&self.fee_payer),
                        &rpc.get_latest_blockhash().await?,
                    );
                    tx = Transaction::new_unsigned(message);
                }
                Err(error) => {
                    tracing::warn!("{}, skipping priority fee", error);
                }
            }
        }

        let msg: Bytes = tx.message_data().into();

        tracing::info!("executing transaction");
        tracing::info!("message size: {}", msg.len());
        tracing::info!("fee payer: {}", self.fee_payer);

        let fee_payer_signature = {
            let keypair = self
                .signers
                .iter()
                .find(|w| w.pubkey() == self.fee_payer)
                .ok_or_else(|| Error::other("fee payer is not in signers"))?;

            if keypair.is_adapter_wallet() {
                let fut = signer.call_ref(signer::SignatureRequest {
                    id: None,
                    time: Utc::now(),
                    pubkey: keypair.pubkey(),
                    message: msg.clone(),
                    timeout: SIGNATURE_TIMEOUT,
                    flow_run_id,
                    signatures: None,
                });
                tokio::time::timeout(SIGNATURE_TIMEOUT, fut)
                    .await
                    .map_err(|_| Error::Timeout)?
                    .map_err(|error| Error::other(error))?
                    .signature
            } else {
                keypair.sign_message(&msg)
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
                message: msg.clone(),
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

        {
            let mut presigners = wallets
                .iter()
                .zip(sigs.iter())
                .map(|(pk, sig)| Presigner::new(pk, &sig.signature))
                .collect::<Vec<_>>();
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

            tx.try_sign(&signers, tx.message.recent_blockhash)?;
        }

        // TODO: is it correct to use FeatureSet::all_enabled()?
        verify_precompiles(&tx, &FeatureSet::all_enabled())?;

        let commitment = CommitmentConfig::confirmed();
        let sig = rpc
            .send_and_confirm_transaction_with_spinner_and_commitment(&tx, commitment)
            .await?;

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
