use super::{CP_AMM_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator, derive_reward_vault};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "fund_reward";
const DEFINITION: &str = flow_lib::node_definition!("damm_v2/fund_reward.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

/// Instruction arguments for `fund_reward`.
#[derive(Serialize, Deserialize, Debug, borsh::BorshSerialize)]
pub struct FundRewardArgs {
    pub reward_index: u8,
    pub amount: u64,
    pub carry_forward: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub funder_token_account: Pubkey,
    #[serde(flatten)]
    pub args: FundRewardArgs,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub reward_vault: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let reward_vault = derive_reward_vault(&input.pool, input.args.reward_index);

    let event_authority =
        Pubkey::find_program_address(&[b"__event_authority"], &CP_AMM_PROGRAM_ID).0;

    let accounts = vec![
        AccountMeta::new(input.pool, false),   // [0] pool (writable)
        AccountMeta::new(reward_vault, false), // [1] reward_vault (writable)
        AccountMeta::new_readonly(input.reward_mint, false), // [2] reward_mint
        AccountMeta::new(input.funder_token_account, false), // [3] funder_token_account (writable)
        AccountMeta::new_readonly(input.payer.pubkey(), true), // [4] funder (signer)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // [5] token_program
        AccountMeta::new_readonly(event_authority, false), // [6] event_authority
        AccountMeta::new_readonly(CP_AMM_PROGRAM_ID, false), // [7] program
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
        fee_payer: input.payer.pubkey(),
        signers: [input.payer].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output {
        signature,
        reward_vault,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_signer::Signer;

    /// Tests that the node definition can be built correctly.
    #[test]
    fn test_build() {
        build().unwrap();
    }

    /// Tests that all required inputs can be parsed from value::map.
    /// Required fields: payer, pool, reward_mint, funder_token_account, reward_index, amount, carry_forward
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "funder_token_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_index" => 0_u8,
            "amount" => 1000u64,
            "carry_forward" => false,
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
            reward_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            funder_token_account: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            args: FundRewardArgs {
                reward_index: 0,
                amount: 1000,
                carry_forward: false,
            },
            submit: false,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        let output = result.unwrap();
        println!("{} output: {:?}", NAME, output);
    }
}
