use super::{
    REWARDS_PROGRAM_ID, RewardsDiscriminator, build_rewards_instruction, default_token_program, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "distribute_continuous_reward";
const DEFINITION: &str = flow_lib::node_definition!("rewards/distribute_continuous_reward.jsonc");

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
    pub authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub reward_pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    #[serde(default = "default_token_program")]
    pub reward_token_program: Pubkey,
    pub amount: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (reward_vault, _) = pda::find_ata(
        &input.reward_pool,
        &input.reward_mint,
        &input.reward_token_program,
    );
    let authority_token_account = pda::find_ata(
        &input.authority.pubkey(),
        &input.reward_mint,
        &input.reward_token_program,
    )
    .0;
    let (event_authority, _) = pda::find_event_authority();

    let accounts = vec![
        AccountMeta::new_readonly(input.authority.pubkey(), true),
        AccountMeta::new(input.reward_pool, false),
        AccountMeta::new_readonly(input.reward_mint, false),
        AccountMeta::new(reward_vault, false),
        AccountMeta::new(authority_token_account, false),
        AccountMeta::new_readonly(input.reward_token_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(REWARDS_PROGRAM_ID, false),
    ];

    let mut args_data = Vec::with_capacity(8);
    args_data.extend_from_slice(&input.amount.to_le_bytes());

    let instruction = build_rewards_instruction(
        RewardsDiscriminator::DistributeContinuousReward,
        accounts,
        args_data,
    );

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.authority.pubkey(),
        signers: [input.authority].into_iter().collect(),
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
            "authority" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "reward_pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "amount" => 1000u64,
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
