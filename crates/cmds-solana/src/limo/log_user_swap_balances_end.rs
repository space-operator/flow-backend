use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{LIMO_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "log_user_swap_balances_end";
const DEFINITION: &str = flow_lib::node_definition!("limo/log_user_swap_balances_end.jsonc");

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
    pub base_accounts: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub user_swap_balance_state: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub event_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub program: Pubkey,
    pub simulated_swap_amount_out: u64,
    pub simulated_ts: u64,
    pub minimum_amount_out: u64,
    pub swap_amount_in: u64,
    pub simulated_amount_out_next_best: u64,
    pub aggregator: u8,
    pub next_best_aggregator: u8,
    pub padding: JsonValue,
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
        AccountMeta::new_readonly(input.base_accounts, false),           // base_accounts
        AccountMeta::new(input.user_swap_balance_state, false),          // user_swap_balance_state (writable)
        AccountMeta::new_readonly(input.event_authority, false),         // event_authority
        AccountMeta::new_readonly(input.program, false),                 // program
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.simulated_swap_amount_out)?);
    data.extend(borsh::to_vec(&input.simulated_ts)?);
    data.extend(borsh::to_vec(&input.minimum_amount_out)?);
    data.extend(borsh::to_vec(&input.swap_amount_in)?);
    data.extend(borsh::to_vec(&input.simulated_amount_out_next_best)?);
    data.extend(borsh::to_vec(&input.aggregator)?);
    data.extend(borsh::to_vec(&input.next_best_aggregator)?);
    let padding_bytes: Vec<u8> = serde_json::from_value(input.padding)?;
    data.extend(borsh::to_vec(&padding_bytes)?);

    let instruction = Instruction {
        program_id: LIMO_PROGRAM_ID,
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
    /// Required fields: fee_payer, base_accounts, user_swap_balance_state, event_authority, program, simulated_swap_amount_out, simulated_ts, minimum_amount_out, swap_amount_in, simulated_amount_out_next_best, aggregator, next_best_aggregator, padding
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "base_accounts" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "user_swap_balance_state" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "event_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "simulated_swap_amount_out" => 1000u64,
            "simulated_ts" => 1000u64,
            "minimum_amount_out" => 1000u64,
            "swap_amount_in" => 1000u64,
            "simulated_amount_out_next_best" => 1000u64,
            "aggregator" => 0_u8,
            "next_best_aggregator" => 0_u8,
            "padding" => serde_json::json!({}),
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
