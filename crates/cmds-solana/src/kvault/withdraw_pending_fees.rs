use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{KVAULT_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "withdraw_pending_fees";
const DEFINITION: &str = flow_lib::node_definition!("kvault/withdraw_pending_fees.jsonc");

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
    pub vault_admin_authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub vault_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reserve: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub ctoken_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_ata: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub lending_market: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub lending_market_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reserve_liquidity_supply: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reserve_collateral_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub klend_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reserve_collateral_token_program: Pubkey,
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
        AccountMeta::new(input.vault_admin_authority.pubkey(), true),    // vault_admin_authority (writable signer)
        AccountMeta::new(input.vault_state, false),                      // vault_state (writable)
        AccountMeta::new(input.reserve, false),                          // reserve (writable)
        AccountMeta::new(input.token_vault, false),                      // token_vault (writable)
        AccountMeta::new(input.ctoken_vault, false),                     // ctoken_vault (writable)
        AccountMeta::new(input.base_vault_authority, false),             // base_vault_authority (writable)
        AccountMeta::new(input.token_ata, false),                        // token_ata (writable)
        AccountMeta::new(input.token_mint, false),                       // token_mint (writable)
        AccountMeta::new_readonly(input.lending_market, false),          // lending_market
        AccountMeta::new_readonly(input.lending_market_authority, false), // lending_market_authority
        AccountMeta::new(input.reserve_liquidity_supply, false),         // reserve_liquidity_supply (writable)
        AccountMeta::new(input.reserve_collateral_mint, false),          // reserve_collateral_mint (writable)
        AccountMeta::new_readonly(input.klend_program, false),           // klend_program
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),              // token_program
        AccountMeta::new_readonly(input.reserve_collateral_token_program, false), // reserve_collateral_token_program
        AccountMeta::new_readonly(input.instruction_sysvar_account, false), // instruction_sysvar_account
    ];

    let data = anchor_discriminator("withdraw_pending_fees").to_vec();

    let instruction = Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.vault_admin_authority].into(),
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
    /// Required fields: fee_payer, vault_admin_authority, vault_state, reserve, token_vault, ctoken_vault, base_vault_authority, token_ata, token_mint, lending_market, lending_market_authority, reserve_liquidity_supply, reserve_collateral_mint, klend_program, reserve_collateral_token_program, instruction_sysvar_account
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "vault_admin_authority" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "vault_state" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reserve" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "ctoken_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_ata" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "lending_market" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "lending_market_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reserve_liquidity_supply" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reserve_collateral_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "klend_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reserve_collateral_token_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "instruction_sysvar_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
