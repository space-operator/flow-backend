use super::{REWARDS_PROGRAM_ID, RewardsDiscriminator, build_rewards_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "close_direct_recipient";
const DEFINITION: &str = flow_lib::node_definition!("rewards/close_direct_recipient.jsonc");

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
    pub original_payer: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub distribution: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (recipient_account, _) =
        pda::find_direct_recipient(&input.distribution, &input.recipient.pubkey());
    let (event_authority, _) = pda::find_event_authority();

    let accounts = vec![
        AccountMeta::new_readonly(input.recipient.pubkey(), true),
        AccountMeta::new(input.original_payer, false),
        AccountMeta::new_readonly(input.distribution, false),
        AccountMeta::new(recipient_account, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(REWARDS_PROGRAM_ID, false),
    ];

    let instruction =
        build_rewards_instruction(RewardsDiscriminator::CloseDirectRecipient, accounts, vec![]);

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
            "recipient" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "original_payer" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "distribution" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
