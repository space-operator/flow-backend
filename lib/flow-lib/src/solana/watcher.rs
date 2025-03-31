use super::{parse_action_memo, parse_rpc_memo_field};
use super::{Pubkey, Signature};
use anyhow::{anyhow, ensure};
use solana_client::{
    client_error, nonblocking::rpc_client::RpcClient,
    rpc_client::GetConfirmedSignaturesForAddress2Config, rpc_request::RpcError,
};
use solana_clock::MAX_HASH_AGE_IN_SECONDS;
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_program::hash::Hash;
use std::time::{Duration, Instant};

pub const ACTION_CONFIRM_TIMEOUT: Duration = Duration::from_secs(60 * 3);

pub async fn confirm_action_transaction(
    rpc: &RpcClient,
    action_identity: Pubkey,
    reference: String,
    level: CommitmentLevel,
) -> Result<Signature, anyhow::Error> {
    let memo = parse_action_memo(&reference)?;
    ensure!(
        memo.identity == action_identity,
        "memo.identity != action_identity"
    );

    let mut until = None;

    let start_time = Instant::now();
    loop {
        let result = rpc
            .get_signatures_for_address_with_config(
                &action_identity,
                GetConfirmedSignaturesForAddress2Config {
                    until,
                    commitment: Some(CommitmentConfig { commitment: level }),
                    ..Default::default()
                },
            )
            .await?;

        let txs = result
            .into_iter()
            .take_while(|r| match r.block_time {
                None => true,
                Some(time) => time >= memo.timestamp,
            })
            .collect::<Vec<_>>();

        if let Some(tx) = txs.first() {
            until = Some(tx.signature.parse()?);
        }

        for tx in txs.into_iter().rev() {
            if let Some(memo) = tx.memo {
                let memos = parse_rpc_memo_field(&memo)?;
                if memos.contains(&reference) {
                    if let Some(err) = tx.err {
                        return Err(err.into());
                    } else {
                        let signature = tx.signature.parse()?;
                        return Ok(signature);
                    }
                }
            }
        }

        if start_time.elapsed() > ACTION_CONFIRM_TIMEOUT {
            return Err(anyhow!("timeout"));
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

// https://docs.rs/solana-rpc-client/2.0.3/src/solana_rpc_client/nonblocking/rpc_client.rs.html#1059-1064
// removed progress bar
pub async fn confirm_transaction(
    rpc: &RpcClient,
    signature: &Signature,
    recent_blockhash: &Hash,
    commitment: CommitmentConfig,
) -> Result<(), client_error::ClientError> {
    let mut confirmations = 0;

    let now = Instant::now();
    let confirm_transaction_initial_timeout = Duration::from_secs(0);
    let (signature, status) = loop {
        // Get recent commitment in order to count confirmations for successful transactions
        let status = rpc
            .get_signature_status_with_commitment(signature, CommitmentConfig::processed())
            .await?;
        if status.is_none() {
            let blockhash_not_found = !rpc
                .is_blockhash_valid(recent_blockhash, CommitmentConfig::processed())
                .await?;
            if blockhash_not_found && now.elapsed() >= confirm_transaction_initial_timeout {
                break (signature, status);
            }
        } else {
            break (signature, status);
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    if let Some(result) = status {
        if let Err(err) = result {
            return Err(err.into());
        }
    } else {
        return Err(RpcError::ForUser(
            "unable to confirm transaction. \
                                      This can happen in situations such as transaction expiration \
                                      and insufficient fee-payer funds"
                .to_string(),
        )
        .into());
    }
    let now = Instant::now();
    loop {
        // Return when specified commitment is reached
        // Failed transactions have already been eliminated, `is_some` check is sufficient
        if rpc
            .get_signature_status_with_commitment(signature, commitment)
            .await?
            .is_some()
        {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
        confirmations = rpc
            .get_num_blocks_since_signature_confirmation(signature)
            .await
            .unwrap_or(confirmations);
        if now.elapsed().as_secs() >= MAX_HASH_AGE_IN_SECONDS as u64 {
            return Err(
                    RpcError::ForUser("transaction not finalized. \
                                      This can happen when a transaction lands in an abandoned fork. \
                                      Please retry.".to_string()).into(),
                );
        }
    }
}

#[cfg(test)]
mod tests {
    use solana_pubkey::pubkey;

    use crate::SolanaNet;

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_watch() {
        const REF: &str = "solana-action:7wf7HHVG2AG4JEUq6FkZfdzoRmR7nrPtTPSEGhXMUtDG:Gs3vmNfDWen3eX5W4sWkkrxtHRSWYjNEvufyTdXqdrMu:2JQc7ebKw6gcZnMfXSnFAZHbAxPqW37b3sQabjVP6BqGJLi1BbpgGNzhmB8rmLEn5zE2TdGsVRgRv2KzrSMqKtbX";
        let rpc = RpcClient::new(SolanaNet::Devnet.url().to_owned());
        let result = confirm_action_transaction(
            &rpc,
            pubkey!("7wf7HHVG2AG4JEUq6FkZfdzoRmR7nrPtTPSEGhXMUtDG"),
            REF.to_owned(),
            CommitmentLevel::Confirmed,
        )
        .await
        .unwrap();
        dbg!(result);
    }
}
