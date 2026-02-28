use crate::prelude::*;
use flow_lib::solana::Pubkey;
pub use solana_program::instruction::AccountMeta;

pub const CROSSBAR_URL: &str = "https://crossbar.switchboard.xyz";

/// Switchboard On-Demand program ID (mainnet).
pub const SB_ON_DEMAND_PID: Pubkey =
    solana_pubkey::pubkey!("SBondMDrcV3K4kxZR1HNVT7osZxAHVHgYXL5Ze1oMUv");

/// Well-known program IDs sourced from their respective crates.
pub use spl_associated_token_account_interface::program::ID as SPL_ATA_PROGRAM;
pub use spl_token_interface::native_mint::ID as WSOL_MINT;
pub use spl_token_interface::ID as SPL_TOKEN_PROGRAM;

/// SlotHashes sysvar.
pub const SLOT_HASHES_SYSVAR: Pubkey =
    solana_pubkey::pubkey!("SysvarS1otHashes111111111111111111111111111");

/// Address Lookup Table program.
pub const ALT_PROGRAM: Pubkey =
    solana_pubkey::pubkey!("AddressLookupTab1e1111111111111111111111111");

// ── PDA derivation ──────────────────────────────────────────────────────

/// Derive the Switchboard program state PDA: seeds = ["STATE"].
pub fn state_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"STATE"], program_id).0
}

/// Derive the LUT signer PDA: seeds = ["LutSigner", account_key].
pub fn lut_signer_pda(program_id: &Pubkey, account_key: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"LutSigner", account_key.as_ref()], program_id).0
}

/// Derive oracle randomness stats PDA: seeds = ["OracleRandomnessStats", oracle].
pub fn oracle_randomness_stats_pda(program_id: &Pubkey, oracle: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"OracleRandomnessStats", oracle.as_ref()], program_id).0
}

/// Get the associated token address for a given mint and owner.
pub fn get_ata(mint: &Pubkey, owner: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), SPL_TOKEN_PROGRAM.as_ref(), mint.as_ref()],
        &SPL_ATA_PROGRAM,
    )
    .0
}

// ── Instruction discriminator ───────────────────────────────────────────

/// Compute the 8-byte Anchor instruction discriminator for a given name.
/// This is SHA256("global:<name>")[0..8].
pub fn instruction_discriminator(name: &str) -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(format!("global:{name}").as_bytes());
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

/// Build a Switchboard instruction with the given name, accounts, and extra data.
pub fn build_sb_instruction(
    name: &str,
    accounts: Vec<AccountMeta>,
    args_data: &[u8],
) -> Instruction {
    let mut data = instruction_discriminator(name).to_vec();
    data.extend_from_slice(args_data);
    Instruction {
        program_id: SB_ON_DEMAND_PID,
        accounts,
        data,
    }
}

/// Precision divisor for i128 feed values (18 decimal places).
pub const PRECISION: i128 = 1_000_000_000_000_000_000;

/// Standard error handling for Crossbar API responses.
pub async fn check_response(resp: reqwest::Response) -> Result<JsonValue, CommandError> {
    if !resp.status().is_success() {
        return Err(CommandError::msg(format!(
            "Switchboard Crossbar API error: {} {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }
    Ok(resp.json().await?)
}

// ── On-chain account parsing ────────────────────────────────────────────
//
// Switchboard PullFeedAccountData is an Anchor zero_copy account compiled
// for BPF (max alignment 8). Rather than reproducing the full struct with
// bytemuck (which fails on x86_64 due to i128 alignment differences), we
// compute the byte offset to the `result: CurrentResult` field and parse
// only that.
//
// PullFeedAccountData BPF layout (offsets from after 8-byte discriminator):
//   0..2048     submissions: [OracleSubmission; 32]   (64 bytes each)
//   2048..2080  authority: Pubkey
//   2080..2112  queue: Pubkey
//   2112..2144  feed_hash: [u8; 32]
//   2144..2152  initialized_at: i64
//   2152..2160  permissions: u64
//   2160..2168  max_variance: u64
//   2168..2172  min_responses: u32
//   2172..2204  name: [u8; 32]
//   2204        permit_write_by_authority: u8
//   2205        historical_result_idx: u8
//   2206        min_sample_size: u8
//   2207        _padding: u8
//   2208..2216  last_update_timestamp: i64
//   2216..2224  lut_slot: u64
//   2224..2352  result: CurrentResult (128 bytes)  <-- what we read
//   2352..2356  max_staleness: u32
//   ...

/// Byte offset from start of struct data (after discriminator) to `result`.
const RESULT_OFFSET: usize = 2224;

/// Anchor discriminator for PullFeedAccountData.
/// SHA256("account:PullFeedAccountData")[0..8]
pub fn pull_feed_discriminator() -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(b"account:PullFeedAccountData");
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

/// Parsed current result from a Switchboard pull feed.
#[derive(Debug)]
pub struct CurrentResult {
    pub value: i128,
    pub std_dev: i128,
    pub mean: i128,
    pub range: i128,
    pub min_value: i128,
    pub max_value: i128,
    pub num_samples: u8,
    pub slot: u64,
    pub min_slot: u64,
    pub max_slot: u64,
}

/// Parse the `CurrentResult` out of raw Switchboard pull feed account data.
///
/// Validates the 8-byte Anchor discriminator, then reads `CurrentResult`
/// at its known byte offset using little-endian decoding.
pub fn parse_pull_feed_result(data: &[u8]) -> Result<CurrentResult, CommandError> {
    const DISC_SIZE: usize = 8;
    const RESULT_END: usize = DISC_SIZE + RESULT_OFFSET + 128;

    if data.len() < RESULT_END {
        return Err(CommandError::msg(format!(
            "Account data too small for PullFeedAccountData: {} bytes (need >= {RESULT_END})",
            data.len(),
        )));
    }

    let disc = pull_feed_discriminator();
    if data[..DISC_SIZE] != disc {
        return Err(CommandError::msg(
            "Invalid Switchboard PullFeed account discriminator",
        ));
    }

    let r = &data[DISC_SIZE + RESULT_OFFSET..];

    // CurrentResult BPF layout (128 bytes):
    //   0..16   value: i128
    //  16..32   std_dev: i128
    //  32..48   mean: i128
    //  48..64   range: i128
    //  64..80   min_value: i128
    //  80..96   max_value: i128
    //  96       num_samples: u8
    //  97       submission_idx: u8
    //  98..104  _padding: [u8; 6]
    // 104..112  slot: u64
    // 112..120  min_slot: u64
    // 120..128  max_slot: u64

    let read_i128 = |off: usize| -> i128 { i128::from_le_bytes(r[off..off + 16].try_into().unwrap()) };
    let read_u64 = |off: usize| -> u64 { u64::from_le_bytes(r[off..off + 8].try_into().unwrap()) };

    Ok(CurrentResult {
        value: read_i128(0),
        std_dev: read_i128(16),
        mean: read_i128(32),
        range: read_i128(48),
        min_value: read_i128(64),
        max_value: read_i128(80),
        num_samples: r[96],
        slot: read_u64(104),
        min_slot: read_u64(112),
        max_slot: read_u64(120),
    })
}
