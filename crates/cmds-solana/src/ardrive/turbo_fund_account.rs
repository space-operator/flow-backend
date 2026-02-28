//! Turbo Fund Account - Fund an ArDrive Turbo account with SOL.
//!
//! Flow:
//! 1. GET {turbo_url}/info → extract Turbo's SOL payment address
//! 2. Build SystemProgram.transfer to that address
//! 3. Optionally add a Memo instruction to direct credits to a different wallet
//! 4. Sign and submit via ctx.execute()
//! 5. POST {turbo_url}/account/balance/solana with tx_id to notify Turbo
//!
//! ArDrive Turbo API: https://payment.ardrive.io/v1

use crate::prelude::*;
use flow_lib::solana::Wallet;
use solana_instruction::{AccountMeta, Instruction};
use solana_system_interface::instruction as system_instruction;
use tracing::info;

/// SPL Memo program ID: MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr
const MEMO_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

/// Build a memo instruction using solana-instruction v3 types directly.
fn build_memo(memo: &[u8], signer_pubkeys: &[&Pubkey]) -> Instruction {
    Instruction {
        program_id: MEMO_PROGRAM_ID,
        accounts: signer_pubkeys
            .iter()
            .map(|&pk| AccountMeta::new_readonly(*pk, true))
            .collect(),
        data: memo.to_vec(),
    }
}

pub const NAME: &str = "turbo_fund_account";
const DEFINITION: &str = flow_lib::node_definition!("ardrive/turbo_fund_account.jsonc");

const TURBO_URL: &str = "https://payment.ardrive.io/v1";

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    pub amount: u64,
    #[serde(default, with = "value::pubkey::opt")]
    pub credit_destination: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    // 1. Fetch the Turbo SOL payment address
    let info_url = format!("{TURBO_URL}/info");
    info!("Fetching Turbo payment info from {}", info_url);

    let resp = ctx.http().get(&info_url).send().await?;
    if !resp.status().is_success() {
        return Err(CommandError::msg(format!(
            "Turbo API /info error: {} {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }

    let info: JsonValue = resp.json().await?;
    let turbo_address_str = info
        .get("addresses")
        .and_then(|a| a.get("solana"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            CommandError::msg(format!(
                "Turbo API did not return a Solana payment address. Response: {info}"
            ))
        })?;

    let turbo_address: Pubkey = turbo_address_str
        .parse()
        .map_err(|e| CommandError::msg(format!("Invalid Turbo Solana address: {e}")))?;

    info!(
        "Turbo payment address: {}, amount: {} lamports",
        turbo_address, input.amount
    );

    // 2. Build SOL transfer instruction
    let payer_pubkey = input.fee_payer.pubkey();
    let ix = system_instruction::transfer(&payer_pubkey, &turbo_address, input.amount);
    let mut ixs = vec![ix];

    // 3. Optionally add memo to direct credits to a different wallet
    if let Some(dest) = input.credit_destination {
        let memo_data = format!("turboCreditDestinationAddress={dest}");
        ixs.push(build_memo(memo_data.as_bytes(), &[]));
        info!("Credits will be directed to {}", dest);
    }

    // 4. Execute the transaction
    let instructions = Instructions {
        lookup_tables: None,
        fee_payer: payer_pubkey,
        signers: [input.fee_payer].into(),
        instructions: ixs,
    };

    let instructions = if input.submit {
        instructions
    } else {
        Default::default()
    };

    let sig_response = ctx
        .execute(
            instructions,
            value::map! {
                "turbo_address" => turbo_address,
            },
        )
        .await?;
    let signature = sig_response.signature;

    // 5. Notify Turbo about the transaction so it processes credits faster
    if let Some(ref sig) = signature {
        let notify_url = format!("{TURBO_URL}/account/balance/solana");
        info!("Notifying Turbo at {} with tx_id {}", notify_url, sig);

        let notify_resp = ctx
            .http()
            .post(&notify_url)
            .json(&serde_json::json!({ "tx_id": sig.to_string() }))
            .send()
            .await;

        match notify_resp {
            Ok(resp) if resp.status().is_success() => {
                let balance: JsonValue = resp.json().await.unwrap_or_default();
                info!("Turbo balance response: {}", balance);
            }
            Ok(resp) => {
                // Non-fatal: the Turbo service will still detect the payment eventually
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                tracing::warn!("Turbo notify returned {status}: {body}");
            }
            Err(e) => {
                tracing::warn!("Failed to notify Turbo (non-fatal): {e}");
            }
        }
    }

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
