use super::{
    REWARDS_PROGRAM_ID, RevokeMode, RewardsDiscriminator, build_rewards_instruction,
    default_token_program, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "revoke_continuous_user";
const DEFINITION: &str = flow_lib::node_definition!("rewards/revoke_continuous_user.jsonc");

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
    pub payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub reward_pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub user: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub rent_destination: Pubkey,
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
    pub revoke_mode: JsonValue,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub revocation_marker: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (user_reward_account, _) = pda::find_user_reward_account(&input.reward_pool, &input.user);
    let (revocation_marker, _) = pda::find_revocation_marker(&input.reward_pool, &input.user);
    let user_tracked_token_account = pda::find_ata(
        &input.user,
        &input.tracked_mint,
        &input.tracked_token_program,
    )
    .0;
    let (reward_vault, _) = pda::find_ata(
        &input.reward_pool,
        &input.reward_mint,
        &input.reward_token_program,
    );
    let user_reward_token_account =
        pda::find_ata(&input.user, &input.reward_mint, &input.reward_token_program).0;
    let authority_reward_token_account = pda::find_ata(
        &input.authority.pubkey(),
        &input.reward_mint,
        &input.reward_token_program,
    )
    .0;
    let (event_authority, _) = pda::find_event_authority();

    let revoke_mode: RevokeMode = serde_json::from_value(input.revoke_mode)?;

    let accounts = vec![
        AccountMeta::new_readonly(input.authority.pubkey(), true),
        AccountMeta::new(input.payer.pubkey(), true),
        AccountMeta::new(input.reward_pool, false),
        AccountMeta::new(user_reward_account, false),
        AccountMeta::new(revocation_marker, false),
        AccountMeta::new_readonly(input.user, false),
        AccountMeta::new(input.rent_destination, false),
        AccountMeta::new_readonly(user_tracked_token_account, false),
        AccountMeta::new(reward_vault, false),
        AccountMeta::new(user_reward_token_account, false),
        AccountMeta::new(authority_reward_token_account, false),
        AccountMeta::new_readonly(input.tracked_mint, false),
        AccountMeta::new_readonly(input.reward_mint, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(input.tracked_token_program, false),
        AccountMeta::new_readonly(input.reward_token_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(REWARDS_PROGRAM_ID, false),
    ];

    let args_data = borsh::to_vec(&revoke_mode)?;

    let instruction = build_rewards_instruction(
        RewardsDiscriminator::RevokeContinuousUser,
        accounts,
        args_data,
    );

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.authority, input.payer].into_iter().collect(),
        instructions: vec![instruction],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        revocation_marker,
    })
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
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "reward_pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "user" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "rent_destination" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "tracked_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "revoke_mode" => serde_json::json!("NonVested"),
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
