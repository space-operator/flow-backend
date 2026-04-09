use super::{REWARDS_PROGRAM_ID, RewardsDiscriminator, build_rewards_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "set_continuous_merkle_root";
const DEFINITION: &str = flow_lib::node_definition!("rewards/set_continuous_merkle_root.jsonc");

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
    pub merkle_root: JsonValue,
    pub root_version: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (event_authority, _) = pda::find_event_authority();

    let root: [u8; 32] = serde_json::from_value(input.merkle_root)?;

    let accounts = vec![
        AccountMeta::new_readonly(input.authority.pubkey(), true),
        AccountMeta::new(input.reward_pool, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(REWARDS_PROGRAM_ID, false),
    ];

    let mut args_data = Vec::new();
    args_data.extend_from_slice(&root);
    args_data.extend_from_slice(&input.root_version.to_le_bytes());

    let instruction = build_rewards_instruction(
        RewardsDiscriminator::SetContinuousMerkleRoot,
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
            "merkle_root" => serde_json::json!([0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]),
            "root_version" => 1u64,
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    #[test]
    fn test_instruction_construction() {
        let authority = Pubkey::new_unique();
        let reward_pool = Pubkey::new_unique();
        let (event_authority, _) = pda::find_event_authority();

        let root = [42u8; 32];
        let root_version = 5u64;

        let accounts = vec![
            solana_program::instruction::AccountMeta::new_readonly(authority, true),
            solana_program::instruction::AccountMeta::new(reward_pool, false),
            solana_program::instruction::AccountMeta::new_readonly(event_authority, false),
            solana_program::instruction::AccountMeta::new_readonly(
                super::REWARDS_PROGRAM_ID,
                false,
            ),
        ];

        let mut args_data = Vec::new();
        args_data.extend_from_slice(&root);
        args_data.extend_from_slice(&root_version.to_le_bytes());

        let ix = super::build_rewards_instruction(
            super::RewardsDiscriminator::SetContinuousMerkleRoot,
            accounts,
            args_data,
        );

        assert_eq!(ix.program_id, super::REWARDS_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 4);
        // 1-byte discriminator + 32-byte root + 8-byte version = 41
        assert_eq!(ix.data.len(), 41);
        assert_eq!(ix.data[0], 20); // SetContinuousMerkleRoot discriminator
    }
}
