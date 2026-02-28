use crate::prelude::*;
use super::{
    CP_AMM_PROGRAM_ID, POSITION_NFT_ACCOUNT_PREFIX, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID,
    anchor_discriminator, derive_event_authority, derive_pool_authority, derive_position,
    derive_reward_vault,
};
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "claim_reward";
const DEFINITION: &str = flow_lib::node_definition!("damm_v2/claim_reward.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

/// Instruction arguments for `claim_reward`.
#[derive(Serialize, Deserialize, Debug, borsh::BorshSerialize)]
pub struct ClaimRewardArgs {
    /// Which reward slot to claim (0 or 1; DAMM v2 supports 2 reward tokens per pool)
    pub reward_index: u8,
    /// 0 = claim normally (fails if vault is frozen),
    /// 1 = skip transfer if vault is frozen (tx succeeds, reward forfeited)
    pub skip_reward: u8,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub owner: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub position_nft_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub user_token_account: Pubkey,
    #[serde(flatten)]
    pub args: ClaimRewardArgs,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub position: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub position_nft_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward_vault: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let pool_authority = derive_pool_authority();
    let position = derive_position(&input.pool, &input.position_nft_mint);
    let position_nft_account = Pubkey::find_program_address(
        &[
            POSITION_NFT_ACCOUNT_PREFIX,
            input.position_nft_mint.as_ref(),
        ],
        &CP_AMM_PROGRAM_ID,
    )
    .0;
    let reward_vault = derive_reward_vault(&input.pool, &input.reward_mint);
    let event_authority = derive_event_authority();

    let accounts = vec![
        AccountMeta::new(input.owner.pubkey(), true), // owner (writable signer)
        AccountMeta::new_readonly(pool_authority, false), // pool_authority
        AccountMeta::new(input.pool, false),          // pool (writable)
        AccountMeta::new(position, false),            // position (writable)
        AccountMeta::new_readonly(input.position_nft_mint, false), // position_nft_mint
        AccountMeta::new_readonly(position_nft_account, false), // position_nft_account
        AccountMeta::new(reward_vault, false),        // reward_vault (writable)
        AccountMeta::new_readonly(input.reward_mint, false), // reward_mint
        AccountMeta::new(input.user_token_account, false), // user_token_account (writable)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false), // token_program
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false), // system_program
        AccountMeta::new_readonly(event_authority, false), // event_authority
        AccountMeta::new_readonly(CP_AMM_PROGRAM_ID, false), // program
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
        fee_payer: input.owner.pubkey(),
        signers: [input.owner].into(),
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
        position,
        position_nft_account,
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
    /// Required fields: owner, pool, position_nft_mint, reward_mint, user_token_account, reward_index, skip_reward
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "owner" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "position_nft_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "user_token_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_index" => 0_u8,
            "skip_reward" => 0_u8,
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
            owner: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            pool: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            position_nft_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            reward_mint: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            user_token_account: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            args: ClaimRewardArgs {
                reward_index: 0,
                skip_reward: 0,
            },
            submit: false,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        let output = result.unwrap();
        println!("{} output: {:?}", NAME, output);
    }
}
