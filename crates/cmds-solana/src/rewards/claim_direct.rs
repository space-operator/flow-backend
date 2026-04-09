use super::{
    REWARDS_PROGRAM_ID, RewardsDiscriminator, build_rewards_instruction, default_token_program, pda,
};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "claim_direct";
const DEFINITION: &str = flow_lib::node_definition!("rewards/claim_direct.jsonc");

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
    pub recipient: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub distribution: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    #[serde(default = "default_token_program")]
    pub token_program: Pubkey,
    pub amount: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub recipient_account: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (recipient_account, _) =
        pda::find_direct_recipient(&input.distribution, &input.recipient.pubkey());
    let (distribution_vault, _) =
        pda::find_ata(&input.distribution, &input.mint, &input.token_program);
    let recipient_token_account =
        pda::find_ata(&input.recipient.pubkey(), &input.mint, &input.token_program).0;
    let (event_authority, _) = pda::find_event_authority();

    let accounts = vec![
        AccountMeta::new_readonly(input.recipient.pubkey(), true),
        AccountMeta::new(input.distribution, false),
        AccountMeta::new(recipient_account, false),
        AccountMeta::new_readonly(input.mint, false),
        AccountMeta::new(distribution_vault, false),
        AccountMeta::new(recipient_token_account, false),
        AccountMeta::new_readonly(input.token_program, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(REWARDS_PROGRAM_ID, false),
    ];

    let mut args_data = Vec::with_capacity(8);
    args_data.extend_from_slice(&input.amount.to_le_bytes());

    let instruction =
        build_rewards_instruction(RewardsDiscriminator::ClaimDirect, accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.recipient.pubkey(),
        signers: [input.recipient].into_iter().collect(),
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
        recipient_account,
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
            "recipient" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "distribution" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "amount" => 500u64,
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
