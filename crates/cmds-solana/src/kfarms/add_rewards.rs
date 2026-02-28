use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{KFARMS_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "add_rewards";
const DEFINITION: &str = flow_lib::node_definition!("kfarms/add_rewards.jsonc");

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
    pub farm_admin: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub farm_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub farm_vaults_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward_token_ata: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reward_mint: Pubkey,
    pub amount: u64,
    pub reward_index: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let accounts = vec![
        AccountMeta::new(input.farm_admin.pubkey(), true),             // farmAdmin (writable signer)
        AccountMeta::new(input.farm_state, false),                     // farmState (writable)
        AccountMeta::new(input.reward_vault, false),                   // rewardVault (writable)
        AccountMeta::new_readonly(input.farm_vaults_authority, false), // farmVaultsAuthority
        AccountMeta::new(input.reward_token_ata, false),               // rewardTokenAta (writable)
        AccountMeta::new_readonly(input.reward_mint, false),           // rewardMint
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),            // tokenProgram
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.amount)?);
    data.extend(borsh::to_vec(&input.reward_index)?);

    let instruction = Instruction {
        program_id: KFARMS_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.farm_admin].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature })
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
    /// Required fields: fee_payer, farm_admin, farm_state, reward_vault, farm_vaults_authority, reward_token_ata, reward_mint, amount, reward_index
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "farm_admin" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "farm_state" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "farm_vaults_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_token_ata" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reward_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "amount" => 1000u64,
            "reward_index" => 1000u64,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
