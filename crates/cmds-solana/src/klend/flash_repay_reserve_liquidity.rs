use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{KLEND_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator, derive_lending_market_authority};

const NAME: &str = "flash_repay_reserve_liquidity";
const DEFINITION: &str = flow_lib::node_definition!("klend/flash_repay_reserve_liquidity.jsonc");

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
    pub user_transfer_authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub lending_market: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reserve: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reserve_destination_liquidity: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub user_source_liquidity: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reserve_liquidity_fee_receiver: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub sysvar_info: Pubkey,
    pub liquidity_amount: u64,
    pub borrow_instruction_index: u8,
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
        AccountMeta::new(input.user_transfer_authority.pubkey(), true),
        AccountMeta::new_readonly(lending_market_authority, false),
        AccountMeta::new(input.lending_market, false),
        AccountMeta::new(input.reserve, false),
        AccountMeta::new(input.reserve_destination_liquidity, false),
        AccountMeta::new(input.user_source_liquidity, false),
        AccountMeta::new(input.reserve_liquidity_fee_receiver, false),
        AccountMeta::new_readonly(input.sysvar_info, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.liquidity_amount)?);
    data.extend(borsh::to_vec(&input.borrow_instruction_index)?);

    let instruction = Instruction {
        program_id: KLEND_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.user_transfer_authority].into(),
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
    /// Required fields: fee_payer, user_transfer_authority, lending_market_authority, lending_market, reserve, reserve_destination_liquidity, user_source_liquidity, reserve_liquidity_fee_receiver, sysvar_info, liquidity_amount, borrow_instruction_index
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "user_transfer_authority" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "lending_market" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reserve" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reserve_destination_liquidity" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "user_source_liquidity" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reserve_liquidity_fee_receiver" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "sysvar_info" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "liquidity_amount" => 1000u64,
            "borrow_instruction_index" => 0_u8,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
