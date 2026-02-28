use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{KVAULT_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "sell";
const DEFINITION: &str = flow_lib::node_definition!("kvault/sell.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub withdraw_from_available: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub withdraw_from_reserve_accounts: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub event_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub program: Pubkey,
    pub shares_amount: u64,
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
        AccountMeta::new(input.withdraw_from_available, false),          // withdraw_from_available (writable)
        AccountMeta::new(input.withdraw_from_reserve_accounts, false),   // withdraw_from_reserve_accounts (writable)
        AccountMeta::new_readonly(input.event_authority, false),         // event_authority
        AccountMeta::new_readonly(input.program, false),                 // program
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.shares_amount)?);

    let instruction = Instruction {
        program_id: KVAULT_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
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
    /// Required fields: fee_payer, withdraw_from_available, withdraw_from_reserve_accounts, event_authority, program, shares_amount
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "withdraw_from_available" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "withdraw_from_reserve_accounts" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "event_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "shares_amount" => 1000u64,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
