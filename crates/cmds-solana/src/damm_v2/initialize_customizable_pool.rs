use super::{
    CP_AMM_PROGRAM_ID, CUSTOMIZABLE_POOL_PREFIX, POSITION_NFT_ACCOUNT_PREFIX, SYSTEM_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator, derive_event_authority,
    derive_pool_authority, derive_position, derive_token_vault,
};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "initialize_customizable_pool";
const DEFINITION: &str = flow_lib::node_definition!("damm_v2/initialize_customizable_pool.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

/// On-chain PoolFeeParameters: base_fee is [u8; 27], not a named struct.
#[derive(Debug, borsh::BorshSerialize)]
pub struct PoolFeeParameters {
    pub base_fee_data: [u8; 27],
    pub compounding_fee_bps: u16,
    pub padding: u8,
    pub dynamic_fee: Option<DynamicFeeParameters>,
}

#[derive(Debug, borsh::BorshSerialize)]
pub struct DynamicFeeParameters {
    pub bin_step: u16,
    pub bin_step_u128: u128,
    pub filter_period: u16,
    pub decay_period: u16,
    pub reduction_factor: u16,
    pub max_volatility_accumulator: u32,
    pub variable_fee_control: u32,
}

/// Build base_fee_data [u8; 27] from fee schedule params (time scheduler variant).
fn encode_base_fee_time_scheduler(
    cliff_fee_numerator: u64,
    number_of_period: u16,
    period_frequency: u64,
    reduction_factor: u64,
    fee_scheduler_mode: u8,
) -> [u8; 27] {
    let mut buf = [0u8; 27];
    buf[0..8].copy_from_slice(&cliff_fee_numerator.to_le_bytes());
    buf[8..10].copy_from_slice(&number_of_period.to_le_bytes());
    buf[10..18].copy_from_slice(&period_frequency.to_le_bytes());
    buf[18..26].copy_from_slice(&reduction_factor.to_le_bytes());
    buf[26] = fee_scheduler_mode;
    buf
}

fn parse_pool_fees(json: &JsonValue) -> Result<PoolFeeParameters, CommandError> {
    // Handle case where pool_fees arrives as a JSON string rather than an object
    let parsed_owned;
    let json = if let Some(s) = json.as_str() {
        parsed_owned = serde_json::from_str::<JsonValue>(s)
            .map_err(|e| anyhow::anyhow!("failed to parse pool_fees string: {e}"))?;
        &parsed_owned
    } else {
        json
    };
    let base_fee = json
        .get("baseFee")
        .ok_or_else(|| anyhow::anyhow!("missing baseFee"))?;
    let cliff = base_fee
        .get("cliffFeeNumerator")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let periods = base_fee
        .get("numberOfPeriod")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u16;
    let freq = base_fee
        .get("periodFrequency")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let red = base_fee
        .get("reductionFactor")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let mode = base_fee
        .get("feeSchedulerMode")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;

    let base_fee_data = encode_base_fee_time_scheduler(cliff, periods, freq, red, mode);

    let dynamic_fee = json.get("dynamicFee").and_then(|v| {
        if v.is_null() {
            return None;
        }
        Some(DynamicFeeParameters {
            bin_step: v.get("binStep")?.as_u64()? as u16,
            bin_step_u128: v.get("binStepU128")?.as_u64()? as u128,
            filter_period: v.get("filterPeriod")?.as_u64()? as u16,
            decay_period: v.get("decayPeriod")?.as_u64()? as u16,
            reduction_factor: v.get("reductionFactor")?.as_u64()? as u16,
            max_volatility_accumulator: v.get("maxVolatilityAccumulator")?.as_u64()? as u32,
            variable_fee_control: v.get("variableFeeControl")?.as_u64()? as u32,
        })
    });

    Ok(PoolFeeParameters {
        base_fee_data,
        compounding_fee_bps: 0,
        padding: 0,
        dynamic_fee,
    })
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub creator: Pubkey,
    pub position_nft_mint: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub token_a_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub payer_token_a: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub payer_token_b: Pubkey,
    pub pool_fees: JsonValue,
    #[serde(deserialize_with = "super::deserialize_flexible_u128")]
    pub sqrt_min_price: u128,
    #[serde(deserialize_with = "super::deserialize_flexible_u128")]
    pub sqrt_max_price: u128,
    pub has_alpha_vault: bool,
    #[serde(deserialize_with = "super::deserialize_flexible_u128")]
    pub liquidity: u128,
    #[serde(deserialize_with = "super::deserialize_flexible_u128")]
    pub sqrt_price: u128,
    pub activation_type: u8,
    pub collect_fee_mode: u8,
    pub activation_point: Option<u64>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub position: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub position_nft_account: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let position_nft_mint_pubkey = input.position_nft_mint.pubkey();
    let (first_mint, second_mint) = if input.token_a_mint > input.token_b_mint {
        (&input.token_a_mint, &input.token_b_mint)
    } else {
        (&input.token_b_mint, &input.token_a_mint)
    };
    let pool = Pubkey::find_program_address(
        &[
            CUSTOMIZABLE_POOL_PREFIX,
            first_mint.as_ref(),
            second_mint.as_ref(),
        ],
        &CP_AMM_PROGRAM_ID,
    )
    .0;
    let pool_authority = derive_pool_authority();
    let token_a_vault = derive_token_vault(&pool, &input.token_a_mint);
    let token_b_vault = derive_token_vault(&pool, &input.token_b_mint);
    let position = derive_position(&pool, &position_nft_mint_pubkey);
    let position_nft_account = Pubkey::find_program_address(
        &[
            POSITION_NFT_ACCOUNT_PREFIX,
            position_nft_mint_pubkey.as_ref(),
        ],
        &CP_AMM_PROGRAM_ID,
    )
    .0;
    let event_authority = derive_event_authority();

    let accounts = vec![
        AccountMeta::new_readonly(input.creator, true), // [0] creator (signer - needed for initial deposit CPI)
        AccountMeta::new(position_nft_mint_pubkey, true), // [1] position_nft_mint (writable signer)
        AccountMeta::new(position_nft_account, false),  // [2] position_nft_account (writable)
        AccountMeta::new(input.payer.pubkey(), true),   // [3] payer (writable signer)
        AccountMeta::new_readonly(pool_authority, false), // [4] pool_authority
        AccountMeta::new(pool, false),                  // [5] pool (writable, init)
        AccountMeta::new(position, false),              // [6] position (writable, init)
        AccountMeta::new_readonly(input.token_a_mint, false), // [7] token_a_mint
        AccountMeta::new_readonly(input.token_b_mint, false), // [8] token_b_mint
        AccountMeta::new(token_a_vault, false),         // [9] token_a_vault (writable)
        AccountMeta::new(token_b_vault, false),         // [10] token_b_vault (writable)
        AccountMeta::new(input.payer_token_a, false),   // [11] payer_token_a (writable)
        AccountMeta::new(input.payer_token_b, false),   // [12] payer_token_b (writable)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // [13] token_a_program
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // [14] token_b_program
        AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false), // [15] token_2022_program
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false), // [16] system_program
        AccountMeta::new_readonly(event_authority, false), // [17] event_authority
        AccountMeta::new_readonly(CP_AMM_PROGRAM_ID, false), // [18] program
    ];

    let pool_fee_params = parse_pool_fees(&input.pool_fees)?;

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&pool_fee_params)?);
    data.extend(borsh::to_vec(&input.sqrt_min_price)?);
    data.extend(borsh::to_vec(&input.sqrt_max_price)?);
    data.extend(borsh::to_vec(&input.has_alpha_vault)?);
    data.extend(borsh::to_vec(&input.liquidity)?);
    data.extend(borsh::to_vec(&input.sqrt_price)?);
    data.extend(borsh::to_vec(&input.activation_type)?);
    data.extend(borsh::to_vec(&input.collect_fee_mode)?);
    data.extend(borsh::to_vec(&input.activation_point)?);

    let instruction = Instruction {
        program_id: CP_AMM_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.position_nft_mint].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output {
        signature,
        pool,
        pool_authority,
        token_a_vault,
        token_b_vault,
        position,
        position_nft_account,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_signer::Signer;

    /// Tests that the node definition can be built correctly.
    #[test]
    fn test_build() {
        build().unwrap();
    }

    /// Tests that all required inputs can be parsed from value::map.
    /// Required fields: payer, creator, position_nft_mint, token_a_mint, token_b_mint, payer_token_a, payer_token_b, pool_fees, sqrt_min_price, sqrt_max_price, has_alpha_vault, liquidity, sqrt_price, activation_type, collect_fee_mode
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "creator" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "position_nft_mint" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "token_a_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_b_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "payer_token_a" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "payer_token_b" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_fees" => serde_json::json!({}),
            "sqrt_min_price" => 0_u128,
            "sqrt_max_price" => 0_u128,
            "has_alpha_vault" => false,
            "liquidity" => 0_u128,
            "sqrt_price" => 0_u128,
            "activation_type" => 0_u8,
            "collect_fee_mode" => 0_u8,
            "submit" => false,
        };

        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    /// Integration test: constructs Input and calls run().
    /// Requires a funded wallet and network access to pass.
    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_run() {
        use solana_keypair::Keypair;

        let input = Input {
            payer: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            creator: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            position_nft_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            token_a_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            token_b_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            payer_token_a: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            payer_token_b: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            pool_fees: serde_json::json!({}),
            sqrt_min_price: 1000,
            sqrt_max_price: 1000,
            has_alpha_vault: false,
            liquidity: 1000,
            sqrt_price: 1000,
            activation_type: 0,
            collect_fee_mode: 0,
            activation_point: None,
            submit: false,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        let output = result.unwrap();
        println!("{} output: {:?}", NAME, output);
    }
}
