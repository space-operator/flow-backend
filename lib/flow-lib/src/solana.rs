use crate::{context::execute::Error, context::signer, FlowRunId};
use anyhow::{anyhow, bail};
use bytes::Bytes;
use futures::TryStreamExt;
use solana_client::{
    client_error::{ClientError, ClientErrorKind},
    nonblocking::rpc_client::RpcClient,
    rpc_request::{RpcError, RpcResponseErrorData},
    rpc_response::RpcSimulateTransactionResult,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    feature_set::FeatureSet,
    instruction::Instruction,
    message::Message,
    precompiles::verify_if_precompile,
    pubkey::Pubkey,
    signature::{Presigner, Signature},
    signer::{keypair::Keypair, Signer},
    transaction::Transaction,
};
use std::time::Duration;
use tower::ServiceExt;

pub const SIGNATURE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

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
    fn clone_keypair(&self) -> Self;
    fn is_user_wallet(&self) -> bool;
}

impl KeypairExt for Keypair {
    fn clone_keypair(&self) -> Self {
        Self::from_bytes(&self.to_bytes()).unwrap()
    }

    fn is_user_wallet(&self) -> bool {
        self.secret().as_bytes().iter().all(|b| *b == 0)
    }
}

#[derive(Default, Debug)]
pub struct Instructions {
    pub fee_payer: Pubkey,
    pub signers: Vec<Keypair>,
    pub instructions: Vec<Instruction>,
}

impl Instructions {
    pub fn combine(&mut self, next: Self) -> Result<(), Self> {
        if next.fee_payer != self.fee_payer {
            return Err(next);
        }

        // TODO: sort and dedup?
        self.signers.extend(next.signers);

        self.instructions.extend(next.instructions);

        Ok(())
    }

    pub async fn execute(
        self,
        rpc: &RpcClient,
        signer: signer::Svc,
        flow_run_id: Option<FlowRunId>,
    ) -> Result<Signature, Error> {
        let recent_blockhash = rpc.get_latest_blockhash().await?;

        let message = Message::new_with_blockhash(
            &self.instructions,
            Some(&self.fee_payer),
            &recent_blockhash,
        );

        let mut tx = Transaction::new_unsigned(message);

        let msg: Bytes = tx.message_data().into();

        let mut wallets = self
            .signers
            .iter()
            .filter_map(|k| {
                if k.is_user_wallet() {
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
                pubkey,
                message: msg.clone(),
                timeout: SIGNATURE_TIMEOUT,
                flow_run_id,
            })
            .collect::<Vec<_>>();

        let fut = signer
            .call_all(futures::stream::iter(reqs))
            .try_collect::<Vec<_>>();

        let sigs = tokio::time::timeout(SIGNATURE_TIMEOUT, fut)
            .await
            .map_err(|_| Error::Timeout)??;

        {
            let presigners = wallets
                .iter()
                .zip(sigs.iter())
                .map(|(pk, sig)| Presigner::new(pk, &sig.signature))
                .collect::<Vec<_>>();

            let mut signers = Vec::<&dyn Signer>::with_capacity(self.signers.len());

            for p in &presigners {
                signers.push(p);
            }

            for k in &self.signers {
                if !k.is_user_wallet() {
                    signers.push(k);
                }
            }

            tx.try_sign(&signers, recent_blockhash)?;
        }

        // TODO: is it correct to use FeatureSet::all_enabled()?
        verify_precompiles(&tx, &FeatureSet::all_enabled())?;

        let commitment = CommitmentConfig::confirmed();
        tracing::trace!("submitting transaction");
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
