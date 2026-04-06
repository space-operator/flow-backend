use super::{
    REWARDS_PROGRAM_ID, RewardsDiscriminator, VestingSchedule, build_rewards_instruction,
    default_token_program, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "claim_merkle";
const DEFINITION: &str = flow_lib::node_definition!("rewards/claim_merkle.jsonc");

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
    pub claimant: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub distribution: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    #[serde(default = "default_token_program")]
    pub token_program: Pubkey,
    pub total_amount: u64,
    pub amount: u64,
    pub schedule: JsonValue,
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
        pda::find_merkle_claim(&input.distribution, &input.claimant.pubkey());
    let (revocation_marker, _) =
        pda::find_revocation_marker(&input.distribution, &input.claimant.pubkey());
    let (distribution_vault, _) =
        pda::find_ata(&input.distribution, &input.mint, &input.token_program);
    let claimant_token_account =
        pda::find_ata(&input.claimant.pubkey(), &input.mint, &input.token_program).0;
    let (event_authority, _) = pda::find_event_authority();

    let schedule: VestingSchedule = serde_json::from_value(input.schedule)?;
    let proof_bytes: Vec<[u8; 32]> = serde_json::from_value(input.proof)?;

    let accounts = vec![
        AccountMeta::new(input.payer.pubkey(), true),
        AccountMeta::new_readonly(input.claimant.pubkey(), true),
        AccountMeta::new(input.distribution, false),
        AccountMeta::new(claim_account, false),
        AccountMeta::new_readonly(revocation_marker, false),
        AccountMeta::new_readonly(input.mint, false),
        AccountMeta::new(distribution_vault, false),
        AccountMeta::new(claimant_token_account, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(input.token_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(REWARDS_PROGRAM_ID, false),
    ];

    let mut args_data: Vec<u8> = Vec::new();
    args_data.push(claim_bump);
    args_data.extend_from_slice(&input.total_amount.to_le_bytes());
    args_data.extend_from_slice(&input.amount.to_le_bytes());
    args_data.extend_from_slice(&borsh::to_vec(&schedule)?);
    args_data.extend_from_slice(&borsh::to_vec(&proof_bytes)?);

    let instruction =
        build_rewards_instruction(RewardsDiscriminator::ClaimMerkle, accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.claimant].into_iter().collect(),
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
            "claimant" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "distribution" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "total_amount" => 1000u64,
            "amount" => 500u64,
            "schedule" => serde_json::json!({"type": "Immediate"}),
            "proof" => serde_json::json!([]),
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
