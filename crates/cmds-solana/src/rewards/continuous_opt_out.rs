use super::{
    REWARDS_PROGRAM_ID, RewardsDiscriminator, build_rewards_instruction, default_token_program, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "continuous_opt_out";
const DEFINITION: &str = flow_lib::node_definition!("rewards/continuous_opt_out.jsonc");

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
    pub user: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub reward_pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub tracked_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    #[serde(default = "default_token_program")]
    pub tracked_token_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    #[serde(default = "default_token_program")]
    pub reward_token_program: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (user_reward_account, _) =
        pda::find_user_reward_account(&input.reward_pool, &input.user.pubkey());
    let user_tracked_token_account = pda::find_ata(
        &input.user.pubkey(),
        &input.tracked_mint,
        &input.tracked_token_program,
    )
    .0;
    let (reward_vault, _) = pda::find_ata(
        &input.reward_pool,
        &input.reward_mint,
        &input.reward_token_program,
    );
    let user_reward_token_account = pda::find_ata(
        &input.user.pubkey(),
        &input.reward_mint,
        &input.reward_token_program,
    )
    .0;
    let (event_authority, _) = pda::find_event_authority();

    let accounts = vec![
        AccountMeta::new(input.user.pubkey(), true),
        AccountMeta::new(input.reward_pool, false),
        AccountMeta::new(user_reward_account, false),
        AccountMeta::new_readonly(user_tracked_token_account, false),
        AccountMeta::new(reward_vault, false),
        AccountMeta::new(user_reward_token_account, false),
        AccountMeta::new_readonly(input.tracked_mint, false),
        AccountMeta::new_readonly(input.reward_mint, false),
        AccountMeta::new_readonly(input.tracked_token_program, false),
        AccountMeta::new_readonly(input.reward_token_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(REWARDS_PROGRAM_ID, false),
    ];

    let instruction =
        build_rewards_instruction(RewardsDiscriminator::ContinuousOptOut, accounts, vec![]);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.user.pubkey(),
        signers: [input.user].into_iter().collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_input_parsing() {
        let input = value::map! {
            "user" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "reward_pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "tracked_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
