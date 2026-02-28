use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{KLEND_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator, derive_lending_market_authority};

const NAME: &str = "liquidate_obligation_and_redeem_reserve_collateral";
const DEFINITION: &str = flow_lib::node_definition!("klend/liquidate_obligation_and_redeem_reserve_collateral.jsonc");

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
    pub liquidator: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub obligation: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub lending_market: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub repay_reserve: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub repay_reserve_liquidity_supply: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub withdraw_reserve: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub withdraw_reserve_collateral_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub withdraw_reserve_collateral_supply: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub withdraw_reserve_liquidity_supply: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub withdraw_reserve_liquidity_fee_receiver: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub user_source_liquidity: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub user_destination_collateral: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub user_destination_liquidity: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub instruction_sysvar_account: Pubkey,
    pub liquidity_amount: u64,
    pub min_acceptable_received_collateral_amount: u64,
    pub max_allowed_ltv_override_percent: u64,
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
        AccountMeta::new(input.liquidator.pubkey(), true),
        AccountMeta::new(input.obligation, false),
        AccountMeta::new(input.lending_market, false),
        AccountMeta::new_readonly(lending_market_authority, false),
        AccountMeta::new(input.repay_reserve, false),
        AccountMeta::new(input.repay_reserve_liquidity_supply, false),
        AccountMeta::new(input.withdraw_reserve, false),
        AccountMeta::new(input.withdraw_reserve_collateral_mint, false),
        AccountMeta::new(input.withdraw_reserve_collateral_supply, false),
        AccountMeta::new(input.withdraw_reserve_liquidity_supply, false),
        AccountMeta::new(input.withdraw_reserve_liquidity_fee_receiver, false),
        AccountMeta::new(input.user_source_liquidity, false),
        AccountMeta::new(input.user_destination_collateral, false),
        AccountMeta::new(input.user_destination_liquidity, false),
        AccountMeta::new_readonly(input.instruction_sysvar_account, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.liquidity_amount)?);
    data.extend(borsh::to_vec(&input.min_acceptable_received_collateral_amount)?);
    data.extend(borsh::to_vec(&input.max_allowed_ltv_override_percent)?);

    let instruction = Instruction {
        program_id: KLEND_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.liquidator].into(),
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
    /// Required fields: fee_payer, liquidator, obligation, lending_market, lending_market_authority, repay_reserve, repay_reserve_liquidity_supply, withdraw_reserve, withdraw_reserve_collateral_mint, withdraw_reserve_collateral_supply, withdraw_reserve_liquidity_supply, withdraw_reserve_liquidity_fee_receiver, user_source_liquidity, user_destination_collateral, user_destination_liquidity, instruction_sysvar_account, liquidity_amount, min_acceptable_received_collateral_amount, max_allowed_ltv_override_percent
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "liquidator" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "obligation" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "lending_market" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "repay_reserve" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "repay_reserve_liquidity_supply" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "withdraw_reserve" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "withdraw_reserve_collateral_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "withdraw_reserve_collateral_supply" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "withdraw_reserve_liquidity_supply" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "withdraw_reserve_liquidity_fee_receiver" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "user_source_liquidity" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "user_destination_collateral" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "user_destination_liquidity" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "instruction_sysvar_account" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "liquidity_amount" => 1000u64,
            "min_acceptable_received_collateral_amount" => 1000u64,
            "max_allowed_ltv_override_percent" => 1000u64,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
