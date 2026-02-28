use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{CP_AMM_PROGRAM_ID, TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID, anchor_discriminator, derive_pool_authority, derive_token_vault, derive_event_authority};

const NAME: &str = "damm_v2_swap";
const IX_NAME: &str = "swap";
const DEFINITION: &str = flow_lib::node_definition!("damm_v2/swap.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

/// Instruction arguments for `swap`.
#[derive(Serialize, Deserialize, Debug, borsh::BorshSerialize)]
pub struct SwapArgs {
    pub amount_in: u64,
    pub minimum_amount_out: u64,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub input_token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub output_token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_mint: Pubkey,
    #[serde_as(as = "Option<AsPubkey>")]
    pub referral_token_account: Option<Pubkey>,
    #[serde(flatten)]
    pub args: SwapArgs,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub pool_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_vault: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let pool_authority = derive_pool_authority();
    let token_a_vault = derive_token_vault(&input.pool, &input.token_a_mint);
    let token_b_vault = derive_token_vault(&input.pool, &input.token_b_mint);
    let event_authority = derive_event_authority();

    let mut accounts = vec![
        AccountMeta::new(input.payer.pubkey(), true),        // payer (writable signer)
        AccountMeta::new_readonly(pool_authority, false),     // pool_authority
        AccountMeta::new(input.pool, false),                  // pool (writable)
        AccountMeta::new(input.input_token_account, false),   // input_token_account (writable)
        AccountMeta::new(input.output_token_account, false),  // output_token_account (writable)
        AccountMeta::new(token_a_vault, false),               // token_a_vault (writable)
        AccountMeta::new(token_b_vault, false),               // token_b_vault (writable)
        AccountMeta::new_readonly(input.token_a_mint, false), // token_a_mint
        AccountMeta::new_readonly(input.token_b_mint, false), // token_b_mint
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),   // token_program
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),  // system_program
    ];

    if let Some(referral) = input.referral_token_account {
        accounts.push(AccountMeta::new(referral, false));
    }

    accounts.push(AccountMeta::new_readonly(event_authority, false));
    accounts.push(AccountMeta::new_readonly(CP_AMM_PROGRAM_ID, false));

    let mut data = anchor_discriminator(IX_NAME).to_vec();
    data.extend(borsh::to_vec(&input.args)?);

    let instruction = Instruction {
        program_id: CP_AMM_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, pool_authority, token_a_vault, token_b_vault })
}

#[cfg(test)]
mod tests {
    use solana_signer::Signer;
    use super::*;

    /// Tests that the node definition can be built correctly.
    #[test]
    fn test_build() {
        build().unwrap();
    }

    /// Tests that all required inputs can be parsed from value::map.
    /// Required fields: payer, pool, input_token_account, output_token_account, token_a_mint, token_b_mint, amount_in, minimum_amount_out
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "input_token_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "output_token_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_a_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_b_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "amount_in" => 1000u64,
            "minimum_amount_out" => 1000u64,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    /// Integration test: constructs Input and calls run().
    /// Requires a funded wallet and network access to pass.
    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_run() {
        use solana_keypair::Keypair;

        let input = Input {
            payer: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            pool: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            input_token_account: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            output_token_account: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            token_a_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            token_b_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            referral_token_account: None,
            args: SwapArgs {
                amount_in: 1000,
                minimum_amount_out: 1000,
            },
            submit: false,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        let output = result.unwrap();
        println!("{} output: {:?}", NAME, output);
    }
}
