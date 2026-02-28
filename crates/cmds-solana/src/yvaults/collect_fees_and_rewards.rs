use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{YVAULTS_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "collect_fees_and_rewards";
const DEFINITION: &str = flow_lib::node_definition!("yvaults/collect_fees_and_rewards.jsonc");

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
    pub fee_payer: Wallet,
    pub user: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub strategy: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub global_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub tick_array_lower: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub tick_array_upper: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub position: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub raydium_protocol_position_or_base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub position_token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_token_vault_a: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_token_vault_b: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_fee_token_a_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_fee_token_b_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_fee_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward0_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward1_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward2_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_reward_vault0: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_reward_vault1: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_reward_vault2: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub instruction_sysvar_account: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let accounts = vec![
        AccountMeta::new(input.user.pubkey(), true),             // user (writable signer)
        AccountMeta::new(input.strategy, false),                 // strategy (writable)
        AccountMeta::new_readonly(input.global_config, false),   // global_config (readonly)
        AccountMeta::new_readonly(input.base_vault_authority, false), // base_vault_authority (readonly)
        AccountMeta::new(input.pool, false),                     // pool (writable)
        AccountMeta::new(input.tick_array_lower, false),         // tick_array_lower (writable)
        AccountMeta::new(input.tick_array_upper, false),         // tick_array_upper (writable)
        AccountMeta::new(input.position, false),                 // position (writable)
        AccountMeta::new(input.raydium_protocol_position_or_base_vault_authority, false), // raydium_protocol_position_or_base_vault_authority
        AccountMeta::new(input.position_token_account, false),   // position_token_account (writable)
        AccountMeta::new(input.token_a_vault, false),            // token_a_vault (writable)
        AccountMeta::new(input.pool_token_vault_a, false),       // pool_token_vault_a (writable)
        AccountMeta::new(input.token_b_vault, false),            // token_b_vault (writable)
        AccountMeta::new(input.pool_token_vault_b, false),       // pool_token_vault_b (writable)
        AccountMeta::new(input.treasury_fee_token_a_vault, false), // treasury_fee_token_a_vault (writable)
        AccountMeta::new(input.treasury_fee_token_b_vault, false), // treasury_fee_token_b_vault (writable)
        AccountMeta::new_readonly(input.treasury_fee_vault_authority, false), // treasury_fee_vault_authority (readonly)
        AccountMeta::new(input.reward0_vault, false),            // reward0_vault (writable)
        AccountMeta::new(input.reward1_vault, false),            // reward1_vault (writable)
        AccountMeta::new(input.reward2_vault, false),            // reward2_vault (writable)
        AccountMeta::new(input.pool_reward_vault0, false),       // pool_reward_vault0 (writable)
        AccountMeta::new(input.pool_reward_vault1, false),       // pool_reward_vault1 (writable)
        AccountMeta::new(input.pool_reward_vault2, false),       // pool_reward_vault2 (writable)
        AccountMeta::new_readonly(input.token_a_mint, false),    // token_a_mint (readonly)
        AccountMeta::new_readonly(input.token_b_mint, false),    // token_b_mint (readonly)
        AccountMeta::new_readonly(input.pool_program, false),    // pool_program (readonly)
        AccountMeta::new_readonly(input.instruction_sysvar_account, false), // instruction_sysvar_account (readonly)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),      // token_program
    ];

    let data = anchor_discriminator(NAME).to_vec();

    let instruction = Instruction {
        program_id: YVAULTS_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.user].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that the node definition can be built correctly.
    #[test]
    fn test_build() {
        build().unwrap();
    }

    /// Tests that all required inputs can be parsed from value::map.
    /// Required fields: fee_payer, user, strategy, global_config, base_vault_authority, pool, tick_array_lower, tick_array_upper, position, raydium_protocol_position_or_base_vault_authority, position_token_account, token_a_vault, pool_token_vault_a, token_b_vault, pool_token_vault_b, treasury_fee_token_a_vault, treasury_fee_token_b_vault, treasury_fee_vault_authority, reward0_vault, reward1_vault, reward2_vault, pool_reward_vault0, pool_reward_vault1, pool_reward_vault2, token_a_mint, token_b_mint, pool_program, instruction_sysvar_account
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "user" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "strategy" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "global_config" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "tick_array_lower" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "tick_array_upper" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "position" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "raydium_protocol_position_or_base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "position_token_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_a_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_token_vault_a" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_b_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_token_vault_b" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "treasury_fee_token_a_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "treasury_fee_token_b_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "treasury_fee_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward0_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward1_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward2_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_reward_vault0" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_reward_vault1" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_reward_vault2" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_a_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_b_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "instruction_sysvar_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
