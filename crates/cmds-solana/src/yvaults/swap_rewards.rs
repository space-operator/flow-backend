use super::derive_ata;
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{YVAULTS_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "swap_rewards";
const DEFINITION: &str = flow_lib::node_definition!("yvaults/swap_rewards.jsonc");

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
    pub token_a_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_fee_token_a_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_fee_token_b_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub treasury_fee_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub user_reward_token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub scope_prices: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_infos: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub instruction_sysvar_account: Pubkey,
    pub token_a_in: u64,
    pub token_b_in: u64,
    pub reward_index: u64,
    pub reward_collateral_id: u64,
    pub min_collateral_token_out: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub user_token_a_ata: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub user_token_b_ata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let user_token_a_ata = derive_ata(&input.user.pubkey(), &input.token_a_mint);
    let user_token_b_ata = derive_ata(&input.user.pubkey(), &input.token_b_mint);

    let accounts = vec![
        AccountMeta::new(input.user.pubkey(), true),             // user (writable signer)
        AccountMeta::new(input.strategy, false),                 // strategy (writable)
        AccountMeta::new_readonly(input.global_config, false),   // global_config (readonly)
        AccountMeta::new(input.pool, false),                     // pool (writable)
        AccountMeta::new(input.token_a_vault, false),            // token_a_vault (writable)
        AccountMeta::new(input.token_b_vault, false),            // token_b_vault (writable)
        AccountMeta::new(input.reward_vault, false),             // reward_vault (writable)
        AccountMeta::new_readonly(input.base_vault_authority, false), // base_vault_authority (readonly)
        AccountMeta::new(input.treasury_fee_token_a_vault, false), // treasury_fee_token_a_vault (writable)
        AccountMeta::new(input.treasury_fee_token_b_vault, false), // treasury_fee_token_b_vault (writable)
        AccountMeta::new_readonly(input.treasury_fee_vault_authority, false), // treasury_fee_vault_authority (readonly)
        AccountMeta::new_readonly(input.token_a_mint, false),    // token_a_mint (readonly)
        AccountMeta::new_readonly(input.token_b_mint, false),    // token_b_mint (readonly)
        AccountMeta::new(user_token_a_ata, false),         // user_token_a_ata (writable)
        AccountMeta::new(user_token_b_ata, false),         // user_token_b_ata (writable)
        AccountMeta::new(input.user_reward_token_account, false), // user_reward_token_account (writable)
        AccountMeta::new_readonly(input.scope_prices, false),    // scope_prices (readonly)
        AccountMeta::new_readonly(input.token_infos, false),     // token_infos (readonly)
        AccountMeta::new_readonly(input.instruction_sysvar_account, false), // instruction_sysvar_account (readonly)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),      // token_program
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.token_a_in)?);
    data.extend(borsh::to_vec(&input.token_b_in)?);
    data.extend(borsh::to_vec(&input.reward_index)?);
    data.extend(borsh::to_vec(&input.reward_collateral_id)?);
    data.extend(borsh::to_vec(&input.min_collateral_token_out)?);

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
    Ok(Output { signature, user_token_a_ata, user_token_b_ata })
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
    /// Required fields: fee_payer, user, strategy, global_config, pool, token_a_vault, token_b_vault, reward_vault, base_vault_authority, treasury_fee_token_a_vault, treasury_fee_token_b_vault, treasury_fee_vault_authority, token_a_mint, token_b_mint, user_reward_token_account, scope_prices, token_infos, instruction_sysvar_account, token_a_in, token_b_in, reward_index, reward_collateral_id, min_collateral_token_out
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "user" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "strategy" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "global_config" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_a_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_b_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "treasury_fee_token_a_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "treasury_fee_token_b_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "treasury_fee_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_a_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_b_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "user_reward_token_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "scope_prices" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_infos" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "instruction_sysvar_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_a_in" => 1000u64,
            "token_b_in" => 1000u64,
            "reward_index" => 1000u64,
            "reward_collateral_id" => 1000u64,
            "min_collateral_token_out" => 1000u64,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
