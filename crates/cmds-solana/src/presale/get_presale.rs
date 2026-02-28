use crate::prelude::*;

const NAME: &str = "get_presale";
const DEFINITION: &str = flow_lib::node_definition!("presale/get_presale.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde_as(as = "AsPubkey")]
    pub presale: Pubkey,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde_as(as = "AsPubkey")]
    pub owner: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub quote_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_token_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub quote_token_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base: Pubkey,
    pub presale_mode: u8,
    pub whitelist_mode: u8,
    pub presale_maximum_cap: u64,
    pub presale_minimum_cap: u64,
    pub presale_start_time: u64,
    pub presale_end_time: u64,
    pub presale_supply: u64,
    pub total_deposit: u64,
    pub total_escrow: u64,
    pub created_at: u64,
    pub lock_duration: u64,
    pub vest_duration: u64,
    pub immediate_release_bps: u16,
    pub vesting_start_time: u64,
    pub vesting_end_time: u64,
    pub total_claimed_token: u64,
    pub total_refunded_quote_token: u64,
    pub has_creator_withdrawn: bool,
    pub unsold_token_action: u8,
    pub is_unsold_token_action_performed: bool,
}

/// Read a u64 from a byte slice at a given offset (little-endian).
fn read_u64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

/// Read a u16 from a byte slice at a given offset (little-endian).
fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let rpc = ctx.solana_client();

    let account = rpc
        .get_account(&input.presale)
        .await
        .map_err(|e| CommandError::msg(format!("Failed to fetch presale account: {}", e)))?;

    // Presale is zero-copy (bytemuck), read raw bytes after 8-byte Anchor discriminator
    // Layout (all offsets relative to discriminator end):
    //   0: owner (Pubkey, 32)
    //  32: quote_mint (Pubkey, 32)
    //  64: base_mint (Pubkey, 32)
    //  96: base_token_vault (Pubkey, 32)
    // 128: quote_token_vault (Pubkey, 32)
    // 160: base (Pubkey, 32)
    // 192: padding0 (u8)
    // 193: presale_mode (u8)
    // 194: whitelist_mode (u8)
    // 195: padding1 ([u8; 5])
    // 200: presale_maximum_cap (u64)
    // 208: presale_minimum_cap (u64)
    // 216: presale_start_time (u64)
    // 224: presale_end_time (u64)
    // 232: presale_supply (u64)
    // 240: total_deposit (u64)
    // 248: total_escrow (u64)
    // 256: created_at (u64)
    // 264: lock_duration (u64)
    // 272: vest_duration (u64)
    // 280: immediate_release_timestamp (u64) [not exposed]
    // 288: padding2 (u64)
    // 296: vesting_start_time (u64)
    // 304: vesting_end_time (u64)
    // 312: total_claimed_token (u64)
    // 320: total_refunded_quote_token (u64)
    // 328: total_deposit_fee (u64) [not exposed - internal]
    // 336: deposit_fee_collected (u8) [not exposed]
    // 337: padding3 ([u8; 7])
    // 344: has_creator_withdrawn (u8)
    // 345: base_token_program_flag (u8) [not exposed]
    // 346: quote_token_program_flag (u8) [not exposed]
    // 347: total_presale_registry_count (u8) [not exposed]
    // 348: unsold_token_action (u8)
    // 349: is_unsold_token_action_performed (u8)
    // 350: immediate_release_bps (u16)

    let min_len = 8 + 352; // discriminator + all fields up to immediate_release_bps
    if account.data.len() < min_len {
        return Err(CommandError::msg("Presale account data too short"));
    }

    let d = &account.data[8..]; // skip discriminator

    let read_pubkey = |off: usize| -> Result<Pubkey, CommandError> {
        Pubkey::try_from(&d[off..off + 32])
            .map_err(|_| CommandError::msg(format!("Invalid pubkey at offset {}", off)))
    };

    Ok(Output {
        owner: read_pubkey(0)?,
        quote_mint: read_pubkey(32)?,
        base_mint: read_pubkey(64)?,
        base_token_vault: read_pubkey(96)?,
        quote_token_vault: read_pubkey(128)?,
        base: read_pubkey(160)?,
        presale_mode: d[193],
        whitelist_mode: d[194],
        presale_maximum_cap: read_u64(d, 200),
        presale_minimum_cap: read_u64(d, 208),
        presale_start_time: read_u64(d, 216),
        presale_end_time: read_u64(d, 224),
        presale_supply: read_u64(d, 232),
        total_deposit: read_u64(d, 240),
        total_escrow: read_u64(d, 248),
        created_at: read_u64(d, 256),
        lock_duration: read_u64(d, 264),
        vest_duration: read_u64(d, 272),
        immediate_release_bps: read_u16(d, 350),
        vesting_start_time: read_u64(d, 296),
        vesting_end_time: read_u64(d, 304),
        total_claimed_token: read_u64(d, 312),
        total_refunded_quote_token: read_u64(d, 320),
        has_creator_withdrawn: d[344] != 0,
        unsold_token_action: d[348],
        is_unsold_token_action_performed: d[349] != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "presale" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
