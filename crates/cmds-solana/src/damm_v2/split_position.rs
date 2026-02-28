use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{CP_AMM_PROGRAM_ID, SYSTEM_PROGRAM_ID, anchor_discriminator, derive_position, derive_event_authority, POSITION_NFT_ACCOUNT_PREFIX};

const NAME: &str = "split_position";
const DEFINITION: &str = flow_lib::node_definition!("damm_v2/split_position.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

/// Instruction arguments for `split_position`.
#[derive(Serialize, Deserialize, Debug, borsh::BorshSerialize)]
pub struct SplitPositionArgs {
    pub unlocked_liquidity_percentage: u8,
    pub permanent_locked_liquidity_percentage: u8,
    pub fee_a_percentage: u8,
    pub fee_b_percentage: u8,
    pub reward_0_percentage: u8,
    pub reward_1_percentage: u8,
    pub inner_vesting_liquidity_percentage: u8,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub first_owner: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub first_position_nft_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub second_position_nft_mint: Pubkey,
    pub second_owner: Wallet,
    #[serde(flatten)]
    pub args: SplitPositionArgs,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub first_position: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub first_position_nft_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub second_position: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub second_position_nft_account: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let first_position = derive_position(&input.pool, &input.first_position_nft_mint);
    let first_position_nft_account = Pubkey::find_program_address(
        &[POSITION_NFT_ACCOUNT_PREFIX, input.first_position_nft_mint.as_ref()],
        &CP_AMM_PROGRAM_ID,
    ).0;
    let second_position = derive_position(&input.pool, &input.second_position_nft_mint);
    let second_position_nft_account = Pubkey::find_program_address(
        &[POSITION_NFT_ACCOUNT_PREFIX, input.second_position_nft_mint.as_ref()],
        &CP_AMM_PROGRAM_ID,
    ).0;
    let event_authority = derive_event_authority();

    let accounts = vec![
        AccountMeta::new(input.first_owner.pubkey(), true),                // first_owner (writable signer)
        AccountMeta::new(input.second_owner.pubkey(), true),               // second_owner (signer)
        AccountMeta::new(input.pool, false),                               // pool (writable)
        AccountMeta::new(first_position, false),                           // first_position (writable)
        AccountMeta::new_readonly(input.first_position_nft_mint, false),   // first_position_nft_mint
        AccountMeta::new_readonly(first_position_nft_account, false),      // first_position_nft_account
        AccountMeta::new(second_position, false),                          // second_position (writable)
        AccountMeta::new_readonly(input.second_position_nft_mint, false),  // second_position_nft_mint
        AccountMeta::new_readonly(second_position_nft_account, false),     // second_position_nft_account
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),               // system_program
        AccountMeta::new_readonly(event_authority, false),                 // event_authority
        AccountMeta::new_readonly(CP_AMM_PROGRAM_ID, false),               // program
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.args)?);

    let instruction = Instruction {
        program_id: CP_AMM_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.first_owner.pubkey(),
        signers: [input.first_owner, input.second_owner].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, first_position, first_position_nft_account, second_position, second_position_nft_account })
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
    /// Required fields: first_owner, pool, first_position_nft_mint, second_position_nft_mint, second_owner, unlocked_liquidity_percentage, permanent_locked_liquidity_percentage, fee_a_percentage, fee_b_percentage, reward_0_percentage, reward_1_percentage, inner_vesting_liquidity_percentage
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "first_owner" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "first_position_nft_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "second_position_nft_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "second_owner" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "unlocked_liquidity_percentage" => 0_u8,
            "permanent_locked_liquidity_percentage" => 0_u8,
            "fee_a_percentage" => 0_u8,
            "fee_b_percentage" => 0_u8,
            "reward_0_percentage" => 0_u8,
            "reward_1_percentage" => 0_u8,
            "inner_vesting_liquidity_percentage" => 0_u8,
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
            first_owner: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            pool: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            first_position_nft_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            second_position_nft_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            second_owner: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            args: SplitPositionArgs {
                unlocked_liquidity_percentage: 0,
                permanent_locked_liquidity_percentage: 0,
                fee_a_percentage: 0,
                fee_b_percentage: 0,
                reward_0_percentage: 0,
                reward_1_percentage: 0,
                inner_vesting_liquidity_percentage: 0,
            },
            submit: false,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        let output = result.unwrap();
        println!("{} output: {:?}", NAME, output);
    }
}
