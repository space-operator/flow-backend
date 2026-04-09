use super::{
    REWARDS_PROGRAM_ID, RewardsDiscriminator, build_rewards_instruction, default_token_program, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "claim_continuous_merkle";
const DEFINITION: &str = flow_lib::node_definition!("rewards/claim_continuous_merkle.jsonc");

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
    pub payer: Wallet,
    pub user: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub reward_pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    #[serde(default = "default_token_program")]
    pub reward_token_program: Pubkey,
    pub root_version: u64,
    pub cumulative_amount: u64,
    pub amount: u64,
    pub proof: JsonValue,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub claim_account: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (claim_account, claim_bump) =
        pda::find_merkle_claim(&input.reward_pool, &input.user.pubkey());
    let (revocation_marker, _) =
        pda::find_revocation_marker(&input.reward_pool, &input.user.pubkey());
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

    let proof_bytes: Vec<[u8; 32]> = serde_json::from_value(input.proof)?;

    let accounts = vec![
        AccountMeta::new(input.payer.pubkey(), true),
        AccountMeta::new_readonly(input.user.pubkey(), true),
        AccountMeta::new(input.reward_pool, false),
        AccountMeta::new(claim_account, false),
        AccountMeta::new_readonly(revocation_marker, false),
        AccountMeta::new_readonly(input.reward_mint, false),
        AccountMeta::new(reward_vault, false),
        AccountMeta::new(user_reward_token_account, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(input.reward_token_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(REWARDS_PROGRAM_ID, false),
    ];

    let mut args_data: Vec<u8> = Vec::new();
    args_data.push(claim_bump);
    args_data.extend_from_slice(&input.root_version.to_le_bytes());
    args_data.extend_from_slice(&input.cumulative_amount.to_le_bytes());
    args_data.extend_from_slice(&input.amount.to_le_bytes());
    args_data.extend_from_slice(&borsh::to_vec(&proof_bytes)?);

    let instruction = build_rewards_instruction(
        RewardsDiscriminator::ClaimContinuousMerkle,
        accounts,
        args_data,
    );

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.user].into_iter().collect(),
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
        claim_account,
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
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "user" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "reward_pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "root_version" => 1u64,
            "cumulative_amount" => 1000u64,
            "amount" => 500u64,
            "proof" => serde_json::json!([]),
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
