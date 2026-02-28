use super::derive_ata;
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{KVAULT_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "withdraw_from_available";
const DEFINITION: &str = flow_lib::node_definition!("kvault/withdraw_from_available.jsonc");

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
    pub vault_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub global_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub shares_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub shares_token_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub klend_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub event_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub program: Pubkey,
    pub shares_amount: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub user_token_ata: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub user_shares_ata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {

    let user_token_ata = derive_ata(&input.user.pubkey(), &input.token_mint);
    let user_shares_ata = derive_ata(&input.user.pubkey(), &input.shares_mint);

    let accounts = vec![
        AccountMeta::new(input.user.pubkey(), true),                 // user (writable signer)
        AccountMeta::new(input.vault_state, false),                  // vault_state (writable)
        AccountMeta::new_readonly(input.global_config, false),       // global_config
        AccountMeta::new(input.token_vault, false),                  // token_vault (writable)
        AccountMeta::new_readonly(input.base_vault_authority, false), // base_vault_authority
        AccountMeta::new(user_token_ata, false),               // user_token_ata (writable)
        AccountMeta::new(input.token_mint, false),                   // token_mint (writable)
        AccountMeta::new(user_shares_ata, false),              // user_shares_ata (writable)
        AccountMeta::new(input.shares_mint, false),                  // shares_mint (writable)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),          // token_program
        AccountMeta::new_readonly(input.shares_token_program, false), // shares_token_program
        AccountMeta::new_readonly(input.klend_program, false),       // klend_program
        AccountMeta::new_readonly(input.event_authority, false),     // event_authority
        AccountMeta::new_readonly(input.program, false),             // program
    ];

    let mut data = anchor_discriminator("withdraw_from_available").to_vec();
    data.extend(borsh::to_vec(&input.shares_amount)?);

    let instruction = Instruction {
        program_id: KVAULT_PROGRAM_ID,
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
    Ok(Output { signature, user_token_ata, user_shares_ata })
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
    /// Required fields: fee_payer, user, vault_state, global_config, token_vault, base_vault_authority, token_mint, shares_mint, shares_token_program, klend_program, event_authority, program, shares_amount
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "user" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "vault_state" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "global_config" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "shares_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "shares_token_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "klend_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "event_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "shares_amount" => 1000u64,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
