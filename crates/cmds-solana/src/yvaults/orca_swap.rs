use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{YVAULTS_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "orca_swap";
const DEFINITION: &str = flow_lib::node_definition!("yvaults/orca_swap.jsonc");

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
    pub funder: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub token_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub whirlpool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_owner_account_a: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_vault_a: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_owner_account_b: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_vault_b: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub tick_array0: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub tick_array1: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub tick_array2: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub oracle: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub whirlpool_program: Pubkey,
    pub amount: u64,
    pub other_amount_threshold: u64,
    pub sqrt_price_limit: u128,
    pub amount_specified_is_input: bool,
    pub a_to_b: bool,
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
        AccountMeta::new(input.funder.pubkey(), true),           // funder (writable signer)
        AccountMeta::new_readonly(input.token_authority, false), // token_authority (readonly)
        AccountMeta::new(input.whirlpool, false),                // whirlpool (writable)
        AccountMeta::new(input.token_owner_account_a, false),    // token_owner_account_a (writable)
        AccountMeta::new(input.token_vault_a, false),            // token_vault_a (writable)
        AccountMeta::new(input.token_owner_account_b, false),    // token_owner_account_b (writable)
        AccountMeta::new(input.token_vault_b, false),            // token_vault_b (writable)
        AccountMeta::new(input.tick_array0, false),              // tick_array0 (writable)
        AccountMeta::new(input.tick_array1, false),              // tick_array1 (writable)
        AccountMeta::new(input.tick_array2, false),              // tick_array2 (writable)
        AccountMeta::new(input.oracle, false),                   // oracle (writable)
        AccountMeta::new_readonly(input.whirlpool_program, false), // whirlpool_program (readonly)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),      // token_program
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.amount)?);
    data.extend(borsh::to_vec(&input.other_amount_threshold)?);
    data.extend(borsh::to_vec(&input.sqrt_price_limit)?);
    data.extend(borsh::to_vec(&input.amount_specified_is_input)?);
    data.extend(borsh::to_vec(&input.a_to_b)?);

    let instruction = Instruction {
        program_id: YVAULTS_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.funder].into(),
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
    /// Required fields: fee_payer, funder, token_authority, whirlpool, token_owner_account_a, token_vault_a, token_owner_account_b, token_vault_b, tick_array0, tick_array1, tick_array2, oracle, whirlpool_program, amount, other_amount_threshold, sqrt_price_limit, amount_specified_is_input, a_to_b
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "funder" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "token_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "whirlpool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_owner_account_a" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_vault_a" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_owner_account_b" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_vault_b" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "tick_array0" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "tick_array1" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "tick_array2" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "oracle" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "whirlpool_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "amount" => 1000u64,
            "other_amount_threshold" => 1000u64,
            "sqrt_price_limit" => 0_u128,
            "amount_specified_is_input" => false,
            "a_to_b" => false,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
