use crate::{context::execute::Error, context::signer, UserId};
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
    instruction::Instruction,
    message::Message,
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
    pub minimum_balance_for_rent_exemption: u64,
    pub instructions: Vec<Instruction>,
}

impl Instructions {
    pub fn combine(&mut self, next: Self) -> Result<(), Self> {
        if next.fee_payer != self.fee_payer {
            return Err(next);
        }

        // TODO: is += the right operation
        self.minimum_balance_for_rent_exemption += next.minimum_balance_for_rent_exemption;

        // TODO: sort and dedup?
        self.signers.extend(next.signers);

        self.instructions.extend(next.instructions);

        Ok(())
    }

    pub async fn execute(
        self,
        rpc: &RpcClient,
        signer: signer::Svc,
        user_id: UserId,
    ) -> Result<Signature, Error> {
        let recent_blockhash = rpc.get_latest_blockhash().await?;
        let balance: u64 = rpc.get_balance(&self.fee_payer).await?;

        let message = Message::new_with_blockhash(
            &self.instructions,
            Some(&self.fee_payer),
            &recent_blockhash,
        );

        let needed =
            self.minimum_balance_for_rent_exemption + rpc.get_fee_for_message(&message).await?;

        if balance < needed {
            return Err(Error::InsufficientSolanaBalance { balance, needed });
        }

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
                user_id,
                pubkey,
                message: msg.clone(),
                timeout: SIGNATURE_TIMEOUT,
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
        let commitment = CommitmentConfig::confirmed();
        tracing::trace!("submitting transaction");
        let sig = rpc
            .send_and_confirm_transaction_with_spinner_and_commitment(&tx, commitment)
            .await?;

        Ok(sig)
    }
}
