use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{KLEND_PROGRAM_ID, SYSTEM_PROGRAM_ID, anchor_discriminator, derive_lending_market_authority};

const NAME: &str = "init_lending_market";
const DEFINITION: &str = flow_lib::node_definition!("klend/init_lending_market.jsonc");

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
    pub lending_market_owner: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub lending_market: Pubkey,
    pub quote_currency: JsonValue,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub lending_market_authority: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let lending_market_authority = derive_lending_market_authority(&input.lending_market);

    let accounts = vec![
        AccountMeta::new(input.lending_market_owner.pubkey(), true),
        AccountMeta::new(input.lending_market, false),
        AccountMeta::new_readonly(lending_market_authority, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    let quote_bytes = serde_json::to_vec(&input.quote_currency)?;
    data.extend(borsh::to_vec(&quote_bytes)?);

    let instruction = Instruction {
        program_id: KLEND_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.lending_market_owner].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, lending_market_authority })
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
    /// Required fields: fee_payer, lending_market_owner, lending_market, quote_currency
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "lending_market_owner" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "lending_market" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "quote_currency" => serde_json::json!({}),
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
