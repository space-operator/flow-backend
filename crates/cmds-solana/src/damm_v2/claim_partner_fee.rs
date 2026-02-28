use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{CP_AMM_PROGRAM_ID, TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID, anchor_discriminator, derive_pool_authority, derive_token_vault, derive_event_authority};

const NAME: &str = "claim_partner_fee";
const DEFINITION: &str = flow_lib::node_definition!("damm_v2/claim_partner_fee.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

/// Instruction arguments for `claim_partner_fee`.
#[derive(Serialize, Deserialize, Debug, borsh::BorshSerialize)]
pub struct ClaimPartnerFeeArgs {
    pub max_amount_a: u64,
    pub max_amount_b: u64,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_a_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_b_mint: Pubkey,
    pub partner: Wallet,
    #[serde(flatten)]
    pub args: ClaimPartnerFeeArgs,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
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

    let accounts = vec![
        AccountMeta::new(input.partner.pubkey(), true),            // partner (writable signer)
        AccountMeta::new_readonly(pool_authority, false),          // pool_authority
        AccountMeta::new(input.pool, false),                       // pool (writable)
        AccountMeta::new(token_a_vault, false),                    // token_a_vault (writable)
        AccountMeta::new(token_b_vault, false),                    // token_b_vault (writable)
        AccountMeta::new_readonly(input.token_a_mint, false),      // token_a_mint
        AccountMeta::new_readonly(input.token_b_mint, false),      // token_b_mint
        AccountMeta::new(input.token_a_account, false),            // token_a_account (writable)
        AccountMeta::new(input.token_b_account, false),            // token_b_account (writable)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),        // token_program
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),       // system_program
        AccountMeta::new_readonly(event_authority, false),         // event_authority
        AccountMeta::new_readonly(CP_AMM_PROGRAM_ID, false),       // program
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.args)?);

    let instruction = Instruction {
        program_id: CP_AMM_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.partner].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, token_a_vault, token_b_vault })
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
    /// Required fields: fee_payer, pool, token_a_account, token_b_account, token_a_mint, token_b_mint, partner, max_amount_a, max_amount_b
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_a_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_b_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_a_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_b_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "partner" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "max_amount_a" => 1000u64,
            "max_amount_b" => 1000u64,
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
            fee_payer: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            pool: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            token_a_account: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            token_b_account: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            token_a_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            token_b_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            partner: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            args: ClaimPartnerFeeArgs {
                max_amount_a: 1000,
                max_amount_b: 1000,
            },
            submit: false,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        let output = result.unwrap();
        println!("{} output: {:?}", NAME, output);
    }
}
