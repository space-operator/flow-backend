use super::{
    REWARDS_PROGRAM_ID, RewardsDiscriminator, build_rewards_instruction, default_token_program, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "close_merkle_distribution";
const DEFINITION: &str = flow_lib::node_definition!("rewards/close_merkle_distribution.jsonc");

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
    pub distribution: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    #[serde(default = "default_token_program")]
    pub token_program: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (distribution_vault, _) =
        pda::find_ata(&input.distribution, &input.mint, &input.token_program);
    let authority_token_account =
        pda::find_ata(&input.authority.pubkey(), &input.mint, &input.token_program).0;
    let (event_authority, _) = pda::find_event_authority();

    let accounts = vec![
        AccountMeta::new(input.authority.pubkey(), true),
        AccountMeta::new(input.distribution, false),
        AccountMeta::new_readonly(input.mint, false),
        AccountMeta::new(distribution_vault, false),
        AccountMeta::new(authority_token_account, false),
        AccountMeta::new_readonly(input.token_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(REWARDS_PROGRAM_ID, false),
    ];

    let instruction = build_rewards_instruction(
        RewardsDiscriminator::CloseMerkleDistribution,
        accounts,
        vec![],
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
            "distribution" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
