use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{YVAULTS_PROGRAM_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "update_reward_mapping";
const DEFINITION: &str = flow_lib::node_definition!("yvaults/update_reward_mapping.jsonc");

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
    pub reward_mint: Pubkey,
    pub reward_vault: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_infos: Pubkey,
    pub reward_index: u8,
    pub collateral_token: u8,
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
        AccountMeta::new_readonly(input.reward_mint, false),     // reward_mint (readonly)
        AccountMeta::new(input.reward_vault.pubkey(), true),     // reward_vault (writable signer)
        AccountMeta::new_readonly(input.base_vault_authority, false), // base_vault_authority (readonly)
        AccountMeta::new_readonly(input.token_infos, false),     // token_infos (readonly)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),      // token_program
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),     // system_program
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.reward_index)?);
    data.extend(borsh::to_vec(&input.collateral_token)?);

    let instruction = Instruction {
        program_id: YVAULTS_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.admin_authority, input.reward_vault].into(),
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
    /// Required fields: fee_payer, admin_authority, strategy, global_config, pool, reward_mint, reward_vault, base_vault_authority, token_infos, reward_index, collateral_token
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "admin_authority" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "strategy" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "global_config" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_vault" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_infos" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_index" => 0_u8,
            "collateral_token" => 0_u8,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
