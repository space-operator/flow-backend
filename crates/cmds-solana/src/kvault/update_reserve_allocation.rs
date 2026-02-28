use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{KVAULT_PROGRAM_ID, SYSTEM_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "update_reserve_allocation";
const DEFINITION: &str = flow_lib::node_definition!("kvault/update_reserve_allocation.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub vault_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reserve_collateral_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reserve: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub ctoken_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reserve_collateral_token_program: Pubkey,
    pub weight: u64,
    pub cap: u64,
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
        AccountMeta::new(input.fee_payer.pubkey(), true),                    // signer (writable signer)
        AccountMeta::new(input.vault_state, false),                          // vault_state (writable)
        AccountMeta::new_readonly(input.base_vault_authority, false),        // base_vault_authority
        AccountMeta::new(input.reserve_collateral_mint, false),              // reserve_collateral_mint (writable)
        AccountMeta::new_readonly(input.reserve, false),                     // reserve
        AccountMeta::new(input.ctoken_vault, false),                         // ctoken_vault (writable)
        AccountMeta::new_readonly(input.reserve_collateral_token_program, false), // reserve_collateral_token_program
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),                 // system_program
        AccountMeta::new_readonly(rent, false),                              // rent
    ];

    let mut data = anchor_discriminator("update_reserve_allocation").to_vec();
    data.extend(borsh::to_vec(&input.weight)?);
    data.extend(borsh::to_vec(&input.cap)?);

    let instruction = Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
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
    /// Required fields: fee_payer, vault_state, base_vault_authority, reserve_collateral_mint, reserve, ctoken_vault, reserve_collateral_token_program, weight, cap
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "vault_state" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reserve_collateral_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reserve" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "ctoken_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reserve_collateral_token_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "weight" => 1000u64,
            "cap" => 1000u64,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
