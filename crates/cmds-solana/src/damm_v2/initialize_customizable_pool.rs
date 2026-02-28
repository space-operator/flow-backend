use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{CP_AMM_PROGRAM_ID, TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID, ATA_PROGRAM_ID, CUSTOMIZABLE_POOL_PREFIX, anchor_discriminator, derive_pool_authority, derive_token_vault, derive_position, derive_event_authority, POSITION_NFT_ACCOUNT_PREFIX};

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
    pub sqrt_min_price: u128,
    pub sqrt_max_price: u128,
    pub has_alpha_vault: bool,
    pub liquidity: u128,
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
    let pool = Pubkey::find_program_address(
        &[CUSTOMIZABLE_POOL_PREFIX, input.token_a_mint.as_ref(), input.token_b_mint.as_ref(), input.creator.as_ref()],
        &CP_AMM_PROGRAM_ID,
    ).0;
    let pool_authority = derive_pool_authority();
    let token_a_vault = derive_token_vault(&pool, &input.token_a_mint);
    let token_b_vault = derive_token_vault(&pool, &input.token_b_mint);
    let position = derive_position(&pool, &position_nft_mint_pubkey);
    let position_nft_account = Pubkey::find_program_address(
        &[POSITION_NFT_ACCOUNT_PREFIX, position_nft_mint_pubkey.as_ref()],
        &CP_AMM_PROGRAM_ID,
    ).0;
    let event_authority = derive_event_authority();

    let accounts = vec![
        AccountMeta::new(input.payer.pubkey(), true),              // payer (writable signer)
        AccountMeta::new_readonly(input.creator, false),           // creator
        AccountMeta::new_readonly(pool_authority, false),          // pool_authority
        AccountMeta::new(pool, false),                             // pool (writable, init)
        AccountMeta::new(position, false),                         // position (writable, init)
        AccountMeta::new(position_nft_mint_pubkey, true),          // position_nft_mint (writable signer)
        AccountMeta::new(position_nft_account, false),             // position_nft_account (writable)
        AccountMeta::new_readonly(input.token_a_mint, false),      // token_a_mint
        AccountMeta::new_readonly(input.token_b_mint, false),      // token_b_mint
        AccountMeta::new(token_a_vault, false),                    // token_a_vault (writable)
        AccountMeta::new(token_b_vault, false),                    // token_b_vault (writable)
        AccountMeta::new(input.payer_token_a, false),              // payer_token_a (writable)
        AccountMeta::new(input.payer_token_b, false),              // payer_token_b (writable)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),        // token_program
        AccountMeta::new_readonly(ATA_PROGRAM_ID, false),          // associated_token_program
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),       // system_program
        AccountMeta::new_readonly(event_authority, false),         // event_authority
        AccountMeta::new_readonly(CP_AMM_PROGRAM_ID, false),       // program
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&serde_json::to_string(&input.pool_fees)?)?);
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

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, pool, pool_authority, token_a_vault, token_b_vault, position, position_nft_account })
}

#[cfg(test)]
mod tests {
    use solana_signer::Signer;
    use super::*;

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
