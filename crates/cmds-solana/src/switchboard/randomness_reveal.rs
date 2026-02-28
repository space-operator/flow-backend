use crate::prelude::*;

use super::helper::*;

pub const NAME: &str = "switchboard_randomness_reveal";
const DEFINITION: &str =
    flow_lib::node_definition!("switchboard/switchboard_randomness_reveal.jsonc");

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
    randomness_account: Pubkey,
    #[serde(default)]
    gateway_url: Option<String>,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Debug)]
struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
    random_value: JsonValue,
}

// ── Randomness account data parsing ──────────────────────────────────

// RandomnessAccountData BPF layout (after 8-byte Anchor discriminator):
//   0..32    authority: Pubkey
//  32..64    queue: Pubkey
//  64..96    seed_slothash: [u8; 32]
//  96..104   seed_slot: u64
// 104..136   oracle: Pubkey
// 136..144   reveal_slot: u64
// 144..176   value: [u8; 32]

fn randomness_discriminator() -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(b"account:RandomnessAccountData");
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

struct RandomnessData {
    authority: Pubkey,
    queue: Pubkey,
    seed_slothash: [u8; 32],
    seed_slot: u64,
    oracle: Pubkey,
}

fn parse_randomness_account(data: &[u8]) -> Result<RandomnessData, CommandError> {
    if data.len() < 8 + 176 {
        return Err(CommandError::msg("Account data too small for RandomnessAccountData"));
    }
    let disc = randomness_discriminator();
    if data[..8] != disc {
        return Err(CommandError::msg("Invalid Switchboard Randomness account discriminator"));
    }
    let d = &data[8..];
    let read_pubkey = |off: usize| -> Pubkey {
        Pubkey::new_from_array(d[off..off + 32].try_into().unwrap())
    };
    let read_u64 = |off: usize| -> u64 {
        u64::from_le_bytes(d[off..off + 8].try_into().unwrap())
    };
    let mut seed_slothash = [0u8; 32];
    seed_slothash.copy_from_slice(&d[64..96]);

    Ok(RandomnessData {
        authority: read_pubkey(0),
        queue: read_pubkey(32),
        seed_slothash,
        seed_slot: read_u64(96),
        oracle: read_pubkey(104),
    })
}

// ── Oracle gateway URL parsing ──────────────────────────────────────

// OracleAccountData has `gateway_uri` as a [u8; 128] null-terminated string.
// Its position depends on the full struct layout. We'll read it at a known offset.
// The oracle account layout (after 8-byte discriminator):
//   0..32   oracle key (or authority)
//   ... many fields ...
// Rather than parsing the whole thing, we search for the gateway URL pattern.

fn extract_gateway_url_from_oracle(data: &[u8]) -> Result<String, CommandError> {
    // The gateway_uri is typically "https://" or "http://" stored as UTF-8.
    // We scan the account data for it.
    let data_str = String::from_utf8_lossy(data);
    if let Some(start) = data_str.find("https://").or_else(|| data_str.find("http://")) {
        let rest = &data_str[start..];
        // Find the end: first null byte or non-printable character
        let end = rest
            .find(|c: char| c == '\0' || !c.is_ascii_graphic())
            .unwrap_or(rest.len());
        let url = &rest[..end];
        if url.len() > 10 {
            return Ok(url.to_string());
        }
    }
    Err(CommandError::msg(
        "Could not extract gateway URL from oracle account data. Pass gateway_url manually.",
    ))
}

// ── Gateway API types ───────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct RandomnessRevealResponse {
    signature: String,   // base64-encoded 64-byte signature
    recovery_id: u8,
    value: Vec<u8>,      // 32 random bytes
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let rpc = ctx.solana_client();

    // 1. Load randomness account data
    let rng_account = rpc
        .get_account(&input.randomness_account)
        .await
        .map_err(|e| CommandError::msg(format!("Failed to fetch randomness account: {e}")))?;
    let rng_data = parse_randomness_account(&rng_account.data)?;

    // 2. Determine gateway URL
    let gateway_url = if let Some(url) = &input.gateway_url {
        url.clone()
    } else {
        let oracle_account = rpc
            .get_account(&rng_data.oracle)
            .await
            .map_err(|e| CommandError::msg(format!("Failed to fetch oracle account: {e}")))?;
        extract_gateway_url_from_oracle(&oracle_account.data)?
    };

    // 3. Call gateway to get randomness reveal
    let rng_key_hex = hex::encode(input.randomness_account.to_bytes());

    let body = serde_json::json!({
        "slothash": rng_data.seed_slothash.iter().map(|b| *b as i32).collect::<Vec<_>>(),
        "randomness_key": rng_key_hex,
        "slot": rng_data.seed_slot,
    });

    let reveal_url = format!(
        "{}/gateway/api/v1/randomness_reveal",
        gateway_url.trim_end_matches('/')
    );

    let resp = ctx
        .http()
        .post(&reveal_url)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(CommandError::msg(format!(
            "Gateway randomness_reveal error: {} {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }

    let reveal: RandomnessRevealResponse = resp.json().await?;

    // 4. Decode signature from base64
    let sig_bytes = base64::decode(&reveal.signature).map_err(|e| {
        CommandError::msg(format!("Failed to decode reveal signature: {e}"))
    })?;
    if sig_bytes.len() != 64 {
        return Err(CommandError::msg(format!(
            "Expected 64-byte signature, got {}",
            sig_bytes.len()
        )));
    }
    let random_value: Vec<u8> = reveal.value;
    if random_value.len() != 32 {
        return Err(CommandError::msg(format!(
            "Expected 32-byte random value, got {}",
            random_value.len()
        )));
    }

    // 5. Build the randomness_reveal instruction
    let payer_pubkey = input.fee_payer.pubkey();
    let program_state = state_pda(&SB_ON_DEMAND_PID);
    let reward_escrow = get_ata(&WSOL_MINT, &input.randomness_account);
    let stats = oracle_randomness_stats_pda(&SB_ON_DEMAND_PID, &rng_data.oracle);

    let accounts = vec![
        AccountMeta::new(input.randomness_account, false),             // randomness (writable)
        AccountMeta::new_readonly(rng_data.oracle, false),             // oracle
        AccountMeta::new_readonly(rng_data.queue, false),              // queue
        AccountMeta::new(stats, false),                                // stats (writable)
        AccountMeta::new_readonly(rng_data.authority, false),          // authority
        AccountMeta::new(payer_pubkey, true),                          // payer (signer, writable)
        AccountMeta::new_readonly(SLOT_HASHES_SYSVAR, false),          // recentSlothashes
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // systemProgram
        AccountMeta::new(reward_escrow, false),                        // rewardEscrow (writable)
        AccountMeta::new_readonly(SPL_TOKEN_PROGRAM, false),           // tokenProgram
        AccountMeta::new_readonly(SPL_ATA_PROGRAM, false),             // associatedTokenProgram
        AccountMeta::new_readonly(WSOL_MINT, false),                   // wrappedSolMint
        AccountMeta::new_readonly(program_state, false),               // programState
    ];

    // Args: { signature: [u8; 64], recovery_id: u8, value: [u8; 32] }
    let mut args_data = Vec::with_capacity(64 + 1 + 32);
    args_data.extend_from_slice(&sig_bytes);
    args_data.push(reveal.recovery_id);
    args_data.extend_from_slice(&random_value);

    let instruction = build_sb_instruction("randomness_reveal", accounts, &args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: payer_pubkey,
        signers: [input.fee_payer].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    // Return the random value as a JSON array of bytes
    let random_value_json = serde_json::json!(random_value);

    Ok(Output {
        signature,
        random_value: random_value_json,
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
