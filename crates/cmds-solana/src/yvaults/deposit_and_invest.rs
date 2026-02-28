use super::derive_ata;
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{YVAULTS_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "deposit_and_invest";
const DEFINITION: &str = flow_lib::node_definition!("yvaults/deposit_and_invest.jsonc");

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
    pub pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub position: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub raydium_protocol_position_or_base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub position_token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_token_vault_a: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_token_vault_b: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub tick_array_lower: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub tick_array_upper: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_fee_token_a_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_fee_token_b_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub shares_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub shares_mint_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub scope_prices: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_infos: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub instruction_sysvar_account: Pubkey,
    pub token_max_a: u64,
    pub token_max_b: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub token_a_ata: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_ata: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub user_shares_ata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let token_a_ata = derive_ata(&input.user.pubkey(), &input.token_a_mint);
    let token_b_ata = derive_ata(&input.user.pubkey(), &input.token_b_mint);
    let user_shares_ata = derive_ata(&input.user.pubkey(), &input.shares_mint);

    let accounts = vec![
        AccountMeta::new(input.user.pubkey(), true),             // user (writable signer)
        AccountMeta::new(input.strategy, false),                 // strategy (writable)
        AccountMeta::new_readonly(input.global_config, false),   // global_config (readonly)
        AccountMeta::new(input.pool, false),                     // pool (writable)
        AccountMeta::new(input.position, false),                 // position (writable)
        AccountMeta::new(input.raydium_protocol_position_or_base_vault_authority, false), // raydium_protocol_position_or_base_vault_authority
        AccountMeta::new(input.position_token_account, false),   // position_token_account (writable)
        AccountMeta::new(input.token_a_vault, false),            // token_a_vault (writable)
        AccountMeta::new(input.token_b_vault, false),            // token_b_vault (writable)
        AccountMeta::new(input.pool_token_vault_a, false),       // pool_token_vault_a (writable)
        AccountMeta::new(input.pool_token_vault_b, false),       // pool_token_vault_b (writable)
        AccountMeta::new(input.tick_array_lower, false),         // tick_array_lower (writable)
        AccountMeta::new(input.tick_array_upper, false),         // tick_array_upper (writable)
        AccountMeta::new_readonly(input.base_vault_authority, false), // base_vault_authority (readonly)
        AccountMeta::new(input.treasury_fee_token_a_vault, false), // treasury_fee_token_a_vault (writable)
        AccountMeta::new(input.treasury_fee_token_b_vault, false), // treasury_fee_token_b_vault (writable)
        AccountMeta::new(token_a_ata, false),              // token_a_ata (writable)
        AccountMeta::new(token_b_ata, false),              // token_b_ata (writable)
        AccountMeta::new_readonly(input.token_a_mint, false),    // token_a_mint (readonly)
        AccountMeta::new_readonly(input.token_b_mint, false),    // token_b_mint (readonly)
        AccountMeta::new(user_shares_ata, false),          // user_shares_ata (writable)
        AccountMeta::new(input.shares_mint, false),              // shares_mint (writable)
        AccountMeta::new_readonly(input.shares_mint_authority, false), // shares_mint_authority (readonly)
        AccountMeta::new_readonly(input.scope_prices, false),    // scope_prices (readonly)
        AccountMeta::new_readonly(input.token_infos, false),     // token_infos (readonly)
        AccountMeta::new_readonly(input.pool_program, false),    // pool_program (readonly)
        AccountMeta::new_readonly(input.instruction_sysvar_account, false), // instruction_sysvar_account (readonly)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),      // token_program
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.token_max_a)?);
    data.extend(borsh::to_vec(&input.token_max_b)?);

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
    Ok(Output { signature, token_a_ata, token_b_ata, user_shares_ata })
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
    /// Required fields: fee_payer, user, strategy, global_config, pool, position, raydium_protocol_position_or_base_vault_authority, position_token_account, token_a_vault, token_b_vault, pool_token_vault_a, pool_token_vault_b, tick_array_lower, tick_array_upper, base_vault_authority, treasury_fee_token_a_vault, treasury_fee_token_b_vault, token_a_mint, token_b_mint, shares_mint, shares_mint_authority, scope_prices, token_infos, pool_program, instruction_sysvar_account, token_max_a, token_max_b
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "user" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "strategy" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "global_config" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "position" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "raydium_protocol_position_or_base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "position_token_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_a_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_b_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_token_vault_a" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_token_vault_b" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "tick_array_lower" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "tick_array_upper" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "treasury_fee_token_a_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "treasury_fee_token_b_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_a_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_b_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "shares_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "shares_mint_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "scope_prices" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_infos" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "instruction_sysvar_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_max_a" => 1000u64,
            "token_max_b" => 1000u64,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
