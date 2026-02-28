use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{YVAULTS_PROGRAM_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "open_liquidity_position";
const DEFINITION: &str = flow_lib::node_definition!("yvaults/open_liquidity_position.jsonc");

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
    pub admin_authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub strategy: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub global_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub tick_array_lower: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub tick_array_upper: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub position: Pubkey,
    pub position_mint: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub position_token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub system: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub old_tick_array_lower_or_base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub old_tick_array_upper_or_base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub old_position_or_base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub old_position_mint_or_base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub old_position_token_account_or_base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_token_vault_a: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_token_vault_b: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub scope_prices: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_infos: Pubkey,
    pub tick_lower_index: i64,
    pub tick_upper_index: i64,
    pub bump: u8,
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
        AccountMeta::new(input.admin_authority.pubkey(), true),  // admin_authority (writable signer)
        AccountMeta::new(input.strategy, false),                 // strategy (writable)
        AccountMeta::new_readonly(input.global_config, false),   // global_config (readonly)
        AccountMeta::new(input.pool, false),                     // pool (writable)
        AccountMeta::new(input.tick_array_lower, false),         // tick_array_lower (writable)
        AccountMeta::new(input.tick_array_upper, false),         // tick_array_upper (writable)
        AccountMeta::new_readonly(input.base_vault_authority, false), // base_vault_authority (readonly)
        AccountMeta::new(input.position, false),                 // position (writable)
        AccountMeta::new(input.position_mint.pubkey(), true),    // position_mint (writable signer)
        AccountMeta::new(input.position_token_account, false),   // position_token_account (writable)
        AccountMeta::new_readonly(input.system, false),          // system (readonly)
        AccountMeta::new_readonly(input.pool_program, false),    // pool_program (readonly)
        AccountMeta::new(input.old_tick_array_lower_or_base_vault_authority, false), // old_tick_array_lower_or_base_vault_authority
        AccountMeta::new(input.old_tick_array_upper_or_base_vault_authority, false), // old_tick_array_upper_or_base_vault_authority
        AccountMeta::new(input.old_position_or_base_vault_authority, false), // old_position_or_base_vault_authority
        AccountMeta::new(input.old_position_mint_or_base_vault_authority, false), // old_position_mint_or_base_vault_authority
        AccountMeta::new(input.old_position_token_account_or_base_vault_authority, false), // old_position_token_account_or_base_vault_authority
        AccountMeta::new(input.token_a_vault, false),            // token_a_vault (writable)
        AccountMeta::new(input.token_b_vault, false),            // token_b_vault (writable)
        AccountMeta::new(input.pool_token_vault_a, false),       // pool_token_vault_a (writable)
        AccountMeta::new(input.pool_token_vault_b, false),       // pool_token_vault_b (writable)
        AccountMeta::new_readonly(input.scope_prices, false),    // scope_prices (readonly)
        AccountMeta::new_readonly(input.token_infos, false),     // token_infos (readonly)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),      // token_program
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),     // system_program
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.tick_lower_index)?);
    data.extend(borsh::to_vec(&input.tick_upper_index)?);
    data.extend(borsh::to_vec(&input.bump)?);

    let instruction = Instruction {
        program_id: YVAULTS_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.admin_authority, input.position_mint].into(),
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
    /// Required fields: fee_payer, admin_authority, strategy, global_config, pool, tick_array_lower, tick_array_upper, base_vault_authority, position, position_mint, position_token_account, system, pool_program, old_tick_array_lower_or_base_vault_authority, old_tick_array_upper_or_base_vault_authority, old_position_or_base_vault_authority, old_position_mint_or_base_vault_authority, old_position_token_account_or_base_vault_authority, token_a_vault, token_b_vault, pool_token_vault_a, pool_token_vault_b, scope_prices, token_infos, tick_lower_index, tick_upper_index, bump
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "admin_authority" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "strategy" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "global_config" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "tick_array_lower" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "tick_array_upper" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "position" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "position_mint" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "position_token_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "system" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "old_tick_array_lower_or_base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "old_tick_array_upper_or_base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "old_position_or_base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "old_position_mint_or_base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "old_position_token_account_or_base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_a_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_b_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_token_vault_a" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_token_vault_b" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "scope_prices" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_infos" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "tick_lower_index" => 0_i64,
            "tick_upper_index" => 0_i64,
            "bump" => 0_u8,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
