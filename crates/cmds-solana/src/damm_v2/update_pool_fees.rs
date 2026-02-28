use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{CP_AMM_PROGRAM_ID, SYSTEM_PROGRAM_ID, anchor_discriminator, derive_event_authority};

const NAME: &str = "update_pool_fees";
const DEFINITION: &str = flow_lib::node_definition!("damm_v2/update_pool_fees.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

/// Instruction arguments for `update_pool_fees`.
#[derive(Serialize, Deserialize, Debug, borsh::BorshSerialize)]
pub struct UpdatePoolFeesArgs {
    pub cliff_fee_numerator: Option<u64>,
    pub dynamic_fee_bin_step: Option<u16>,
    pub dynamic_fee_bin_step_u128: Option<u128>,
    pub dynamic_fee_filter_period: Option<u16>,
    pub dynamic_fee_decay_period: Option<u16>,
    pub dynamic_fee_reduction_factor: Option<u16>,
    pub dynamic_fee_max_volatility_accumulator: Option<u32>,
    pub dynamic_fee_variable_fee_control: Option<u32>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub operator: Pubkey,
    pub signer: Wallet,
    #[serde(flatten)]
    pub args: UpdatePoolFeesArgs,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let event_authority = derive_event_authority();

    let accounts = vec![
        AccountMeta::new(input.pool, false),                       // pool (writable)
        AccountMeta::new_readonly(input.operator, false),          // operator
        AccountMeta::new(input.signer.pubkey(), true),             // signer (signer)
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),       // system_program
        AccountMeta::new_readonly(event_authority, false),         // event_authority
        AccountMeta::new_readonly(CP_AMM_PROGRAM_ID, false),       // program
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.args)?);

    let instruction = Instruction {
        program_id: CP_AMM_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.signer].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use solana_signer::Signer;
    use super::*;

    /// Tests that the node definition can be built correctly.
    #[test]
    fn test_build() {
        build().unwrap();
    }

    /// Tests that all required inputs can be parsed from value::map.
    /// Required fields: fee_payer, pool, operator, signer
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "operator" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "signer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }

    /// Integration test: constructs Input and calls run().
    /// Requires a funded wallet and network access to pass.
    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_run() {
        use solana_keypair::Keypair;

        let input = Input {
            fee_payer: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            pool: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            operator: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            signer: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            args: UpdatePoolFeesArgs {
                cliff_fee_numerator: None,
                dynamic_fee_bin_step: Default::default() /* Option<u16> */,
                dynamic_fee_bin_step_u128: Default::default() /* Option<u128> */,
                dynamic_fee_filter_period: Default::default() /* Option<u16> */,
                dynamic_fee_decay_period: Default::default() /* Option<u16> */,
                dynamic_fee_reduction_factor: Default::default() /* Option<u16> */,
                dynamic_fee_max_volatility_accumulator: Default::default() /* Option<u32> */,
                dynamic_fee_variable_fee_control: Default::default() /* Option<u32> */,
            },
            submit: false,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        let output = result.unwrap();
        println!("{} output: {:?}", NAME, output);
    }
}
