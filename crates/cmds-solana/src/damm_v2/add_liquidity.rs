use super::{
    CP_AMM_PROGRAM_ID, POSITION_NFT_ACCOUNT_PREFIX, TOKEN_PROGRAM_ID, anchor_discriminator,
    derive_event_authority, derive_position, derive_token_vault,
};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "add_liquidity";
const DEFINITION: &str = flow_lib::node_definition!("damm_v2/add_liquidity.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

/// Instruction arguments for `add_liquidity`.
#[derive(Serialize, Deserialize, Debug, borsh::BorshSerialize)]
pub struct AddLiquidityArgs {
    pub liquidity_delta: u128,
    pub token_a_amount_threshold: u64,
    pub token_b_amount_threshold: u64,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub owner: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub position_nft_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_mint: Pubkey,
    #[serde(deserialize_with = "super::deserialize_flexible_u128")]
    pub liquidity_delta: u128,
    pub token_a_amount_threshold: u64,
    pub token_b_amount_threshold: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub position: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub position_nft_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_vault: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let position = derive_position(&input.pool, &input.position_nft_mint);
    let position_nft_account = Pubkey::find_program_address(
        &[
            POSITION_NFT_ACCOUNT_PREFIX,
            input.position_nft_mint.as_ref(),
        ],
        &CP_AMM_PROGRAM_ID,
    )
    .0;
    let token_a_vault = derive_token_vault(&input.pool, &input.token_a_mint);
    let token_b_vault = derive_token_vault(&input.pool, &input.token_b_mint);
    let event_authority = derive_event_authority();

    let accounts = vec![
        AccountMeta::new(input.pool, false), // [0] pool (writable)
        AccountMeta::new(position, false),   // [1] position (writable)
        AccountMeta::new(input.token_a_account, false), // [2] token_a_account (writable)
        AccountMeta::new(input.token_b_account, false), // [3] token_b_account (writable)
        AccountMeta::new(token_a_vault, false), // [4] token_a_vault (writable)
        AccountMeta::new(token_b_vault, false), // [5] token_b_vault (writable)
        AccountMeta::new_readonly(input.token_a_mint, false), // [6] token_a_mint
        AccountMeta::new_readonly(input.token_b_mint, false), // [7] token_b_mint
        AccountMeta::new_readonly(position_nft_account, false), // [8] position_nft_account
        AccountMeta::new_readonly(input.owner.pubkey(), true), // [9] owner (readonly signer)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // [10] token_a_program
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // [11] token_b_program
        AccountMeta::new_readonly(event_authority, false), // [12] event_authority
        AccountMeta::new_readonly(CP_AMM_PROGRAM_ID, false), // [13] program
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    let args = AddLiquidityArgs {
        liquidity_delta: input.liquidity_delta,
        token_a_amount_threshold: input.token_a_amount_threshold,
        token_b_amount_threshold: input.token_b_amount_threshold,
    };
    data.extend(borsh::to_vec(&args)?);

    let instruction = Instruction {
        program_id: CP_AMM_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.owner.pubkey(),
        signers: [input.owner].into(),
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
        position,
        position_nft_account,
        token_a_vault,
        token_b_vault,
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
    /// Required fields: owner, pool, position_nft_mint, token_a_account, token_b_account, token_a_mint, token_b_mint, liquidity_delta, token_a_amount_threshold, token_b_amount_threshold
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "owner" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "position_nft_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_a_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_b_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_a_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_b_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_a_amount_threshold" => 1000u64,
            "liquidity_delta" => 0_u128,
            "token_b_amount_threshold" => 1000u64,
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
            owner: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            pool: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            position_nft_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            token_a_account: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            token_b_account: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            token_a_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            token_b_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            liquidity_delta: 1000,

            token_a_amount_threshold: 1000,

            token_b_amount_threshold: 1000,
            submit: false,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        let output = result.unwrap();
        println!("{} output: {:?}", NAME, output);
    }
}
