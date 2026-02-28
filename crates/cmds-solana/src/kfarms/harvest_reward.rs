use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{KFARMS_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator, derive_user_state};

const NAME: &str = "harvest_reward";
const DEFINITION: &str = flow_lib::node_definition!("kfarms/harvest_reward.jsonc");

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
    pub owner: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub farm_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub global_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub user_reward_ata: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub rewards_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub rewards_treasury_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub farm_vaults_authority: Pubkey,
    pub reward_index: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub user_state: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let user_state = derive_user_state(&input.farm_state, &input.owner.pubkey());

    let accounts = vec![
        AccountMeta::new(input.owner.pubkey(), true),              // owner (writable signer)
        AccountMeta::new(user_state, false),                       // userState (writable, PDA)
        AccountMeta::new(input.farm_state, false),                 // farmState (writable)
        AccountMeta::new_readonly(input.global_config, false),     // globalConfig
        AccountMeta::new(input.user_reward_ata, false),            // userRewardAta (writable)
        AccountMeta::new(input.rewards_vault, false),              // rewardsVault (writable)
        AccountMeta::new(input.rewards_treasury_vault, false),     // rewardsTreasuryVault (writable)
        AccountMeta::new_readonly(input.farm_vaults_authority, false), // farmVaultsAuthority
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),        // tokenProgram
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.reward_index)?);

    let instruction = Instruction {
        program_id: KFARMS_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.owner].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, user_state })
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
    /// Required fields: fee_payer, owner, farm_state, global_config, user_reward_ata, rewards_vault, rewards_treasury_vault, farm_vaults_authority, reward_index
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "owner" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "farm_state" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "global_config" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "user_reward_ata" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "rewards_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "rewards_treasury_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "farm_vaults_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_index" => 1000u64,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
