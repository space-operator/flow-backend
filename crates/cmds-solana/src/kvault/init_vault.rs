use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{KVAULT_PROGRAM_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "init_vault";
const DEFINITION: &str = flow_lib::node_definition!("kvault/init_vault.jsonc");

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
    pub vault_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_token_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub shares_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub admin_token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub shares_token_program: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {

    let rent = solana_pubkey::pubkey!("SysvarRent111111111111111111111111111111111");

    let accounts = vec![
        AccountMeta::new(input.admin_authority.pubkey(), true),      // admin_authority (writable signer)
        AccountMeta::new(input.vault_state, false),                  // vault_state (writable)
        AccountMeta::new_readonly(input.base_vault_authority, false), // base_vault_authority
        AccountMeta::new(input.token_vault, false),                  // token_vault (writable)
        AccountMeta::new_readonly(input.base_token_mint, false),     // base_token_mint
        AccountMeta::new(input.shares_mint, false),                  // shares_mint (writable)
        AccountMeta::new(input.admin_token_account, false),          // admin_token_account (writable)
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),         // system_program
        AccountMeta::new_readonly(rent, false),                      // rent
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),          // token_program
        AccountMeta::new_readonly(input.shares_token_program, false), // shares_token_program
    ];

    let data = anchor_discriminator("init_vault").to_vec();

    let instruction = Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.admin_authority].into(),
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
    /// Required fields: fee_payer, admin_authority, vault_state, base_vault_authority, token_vault, base_token_mint, shares_mint, admin_token_account, shares_token_program
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "admin_authority" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "vault_state" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "base_token_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "shares_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "admin_token_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "shares_token_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
