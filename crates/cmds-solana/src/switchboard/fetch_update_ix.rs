use crate::prelude::*;

use super::helper::*;

pub const NAME: &str = "switchboard_fetch_update_ix";
const DEFINITION: &str =
    flow_lib::node_definition!("switchboard/switchboard_fetch_update_ix.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Deserialize, Debug)]
struct Input {
    fee_payer: Wallet,
    #[serde(with = "value::pubkey")]
    feed: Pubkey,
    #[serde(with = "value::pubkey")]
    queue: Pubkey,
    #[serde(default)]
    gateway_url: Option<String>,
    #[serde(default)]
    crossbar_url: Option<String>,
    #[serde(default)]
    num_signatures: Option<u32>,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Debug)]
struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
    simulated_value: JsonValue,
}

// ── Crossbar simulate response ──────────────────────────────────────

#[derive(Deserialize, Serialize, Debug)]
struct SimulateResult {
    feed: Option<String>,
    results: Vec<f64>,
    #[serde(rename = "feedHash")]
    feed_hash: Option<String>,
}

// ── Gateway fetch_signatures response ───────────────────────────────

#[derive(Deserialize, Debug)]
struct OracleSignatureResponse {
    oracle_pubkey: String,
    success_value: String,
    signature: String,  // base64
    recovery_id: u8,
}

#[derive(Deserialize, Debug)]
struct FetchSignaturesResponse {
    oracle_responses: Vec<OracleSignatureResponse>,
    slot: u64,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let payer_pubkey = input.fee_payer.pubkey();
    let crossbar_base = input
        .crossbar_url
        .as_deref()
        .unwrap_or(CROSSBAR_URL);

    // 1. Simulate the feed to get current value
    let simulate_url = format!("{}/simulate/solana", crossbar_base.trim_end_matches('/'));
    let sim_body = serde_json::json!({
        "cluster": "Mainnet",
        "feeds": [input.feed.to_string()],
    });
    let sim_resp = ctx.http().post(&simulate_url).json(&sim_body).send().await?;
    let simulated: Vec<SimulateResult> = check_response(sim_resp)
        .await?
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap_or(SimulateResult {
            feed: None,
            results: vec![],
            feed_hash: None,
        }))
        .collect();

    let simulated_value = serde_json::to_value(&simulated)
        .unwrap_or(serde_json::json!(null));

    // 2. Load feed account to get the feed_hash and queue
    let feed_account = ctx
        .solana_client()
        .get_account(&input.feed)
        .await
        .map_err(|e| CommandError::msg(format!("Failed to fetch feed account: {e}")))?;

    // Extract feed_hash from account data (offset 2112..2144 after 8-byte discriminator)
    if feed_account.data.len() < 8 + 2144 {
        return Err(CommandError::msg("Feed account data too small"));
    }
    let feed_hash = &feed_account.data[8 + 2112..8 + 2144];
    let feed_hash_hex = format!("0x{}", hex::encode(feed_hash));

    // 3. If gateway_url not provided, we need to find an oracle to get it.
    //    For simplicity, use the crossbar gateway endpoint.
    let gateway_base = if let Some(url) = &input.gateway_url {
        url.clone()
    } else {
        // Use crossbar's built-in gateway
        crossbar_base.to_string()
    };

    // 4. Fetch oracle signatures via the gateway
    let num_sigs = input.num_signatures.unwrap_or(1);
    let fetch_url = format!(
        "{}/gateway/api/v1/fetch_signatures",
        gateway_base.trim_end_matches('/')
    );

    let fetch_body = serde_json::json!({
        "feed_hash": feed_hash_hex,
        "queue": input.queue.to_string(),
        "num_signatures": num_sigs,
    });

    let fetch_resp = ctx.http().post(&fetch_url).json(&fetch_body).send().await?;
    if !fetch_resp.status().is_success() {
        return Err(CommandError::msg(format!(
            "Gateway fetch_signatures error: {} {}",
            fetch_resp.status(),
            fetch_resp.text().await.unwrap_or_default()
        )));
    }

    let sig_response: FetchSignaturesResponse = fetch_resp.json().await?;

    // 5. Build the pullFeedSubmitResponse instruction
    let program_state = state_pda(&SB_ON_DEMAND_PID);
    let reward_vault = get_ata(&WSOL_MINT, &input.queue);

    let mut accounts = vec![
        AccountMeta::new(input.feed, false),                           // feed (writable)
        AccountMeta::new_readonly(input.queue, false),                 // queue
        AccountMeta::new_readonly(program_state, false),               // programState
        AccountMeta::new_readonly(SLOT_HASHES_SYSVAR, false),          // recentSlothashes
        AccountMeta::new(payer_pubkey, true),                          // payer (signer, writable)
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // systemProgram
        AccountMeta::new(reward_vault, false),                         // rewardVault (writable)
        AccountMeta::new_readonly(SPL_TOKEN_PROGRAM, false),           // tokenProgram
        AccountMeta::new_readonly(WSOL_MINT, false),                   // tokenMint
    ];

    // Remaining accounts: oracle pubkeys (read-only), then oracle feed stats (writable)
    let mut oracle_keys = Vec::new();
    let mut submissions = Vec::new();

    for resp in &sig_response.oracle_responses {
        let oracle_pubkey: Pubkey = resp.oracle_pubkey.parse().map_err(|_| {
            CommandError::msg(format!("Invalid oracle pubkey: {}", resp.oracle_pubkey))
        })?;
        oracle_keys.push(oracle_pubkey);

        let sig_bytes = base64::decode(&resp.signature).map_err(|e| {
            CommandError::msg(format!("Failed to decode oracle signature: {e}"))
        })?;

        let value: i128 = resp.success_value.parse().map_err(|_| {
            CommandError::msg(format!("Invalid success_value: {}", resp.success_value))
        })?;

        submissions.push((value, sig_bytes, resp.recovery_id));
    }

    // Add oracle accounts (read-only)
    for key in &oracle_keys {
        accounts.push(AccountMeta::new_readonly(*key, false));
    }

    // Add oracle feed stats accounts (writable)
    for key in &oracle_keys {
        let stats = Pubkey::find_program_address(
            &[b"OracleStats", key.as_ref()],
            &SB_ON_DEMAND_PID,
        )
        .0;
        accounts.push(AccountMeta::new(stats, false));
    }

    // Args: { slot: u64, submissions: Vec<Submission> }
    // Submission: { value: i128, signature: [u8; 64], recovery_id: u8, slot_offset: u8 }
    let mut args_data = Vec::new();
    args_data.extend_from_slice(&sig_response.slot.to_le_bytes());
    // Borsh Vec encoding: u32 length prefix + items
    args_data.extend_from_slice(&(submissions.len() as u32).to_le_bytes());
    for (value, sig, recovery_id) in &submissions {
        args_data.extend_from_slice(&value.to_le_bytes());   // i128
        args_data.extend_from_slice(sig);                    // [u8; 64]
        args_data.push(*recovery_id);                        // u8
        args_data.push(0u8);                                 // offset (slot diff, usually 0)
    }

    let instruction = build_sb_instruction("pull_feed_submit_response", accounts, &args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: payer_pubkey,
        signers: [input.fee_payer].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        simulated_value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
