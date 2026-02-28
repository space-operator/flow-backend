use crate::prelude::*;

const NAME: &str = "get_escrow";
const DEFINITION: &str = flow_lib::node_definition!("presale/get_escrow.jsonc");

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
    pub escrow: Pubkey,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde_as(as = "AsPubkey")]
    pub presale: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub owner: Pubkey,
    pub total_deposit: u64,
    pub total_claimed_token: u64,
    pub is_remaining_quote_withdrawn: bool,
    pub registry_index: u8,
    pub pending_claim_token: u64,
    pub deposit_max_cap: u64,
    pub created_at: u64,
    pub total_deposit_fee: u64,
    pub last_refreshed_at: u64,
}

/// Read a u64 from a byte slice at a given offset (little-endian).
fn read_u64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let rpc = ctx.solana_client();

    let account = rpc
        .get_account(&input.escrow)
        .await
        .map_err(|e| CommandError::msg(format!("Failed to fetch escrow account: {}", e)))?;

    // Escrow is zero-copy (bytemuck), read raw bytes after 8-byte Anchor discriminator
    // Layout (all offsets relative to discriminator end):
    //   0: presale (Pubkey, 32)
    //  32: owner (Pubkey, 32)
    //  64: total_deposit (u64, 8)
    //  72: total_claimed_token (u64, 8)
    //  80: is_remaining_quote_withdrawn (u8, 1)
    //  81: registry_index (u8, 1)
    //  82: padding0 ([u8; 6])
    //  88: pending_claim_token (u64, 8)
    //  96: deposit_max_cap (u64, 8)
    // 104: created_at (u64, 8)
    // 112: total_deposit_fee (u64, 8)
    // 120: last_refreshed_at (u64, 8)
    // 128: padding ([u64; 8], 64)

    let min_len = 8 + 128; // discriminator + all fields up to last_refreshed_at end
    if account.data.len() < min_len {
        return Err(CommandError::msg("Escrow account data too short"));
    }

    let d = &account.data[8..]; // skip discriminator

    let presale = Pubkey::try_from(&d[0..32])
        .map_err(|_| CommandError::msg("Invalid presale pubkey"))?;
    let owner = Pubkey::try_from(&d[32..64])
        .map_err(|_| CommandError::msg("Invalid owner pubkey"))?;

    Ok(Output {
        presale,
        owner,
        total_deposit: read_u64(d, 64),
        total_claimed_token: read_u64(d, 72),
        is_remaining_quote_withdrawn: d[80] != 0,
        registry_index: d[81],
        pending_claim_token: read_u64(d, 88),
        deposit_max_cap: read_u64(d, 96),
        created_at: read_u64(d, 104),
        total_deposit_fee: read_u64(d, 112),
        last_refreshed_at: read_u64(d, 120),
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
            "escrow" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
