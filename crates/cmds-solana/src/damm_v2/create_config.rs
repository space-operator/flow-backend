use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{CP_AMM_PROGRAM_ID, SYSTEM_PROGRAM_ID, anchor_discriminator, derive_config, derive_event_authority};

const NAME: &str = "damm_v2_create_config";
const IX_NAME: &str = "create_config";
const DEFINITION: &str = flow_lib::node_definition!("damm_v2/create_config.jsonc");

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
    pub payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub operator: Pubkey,
    pub signer: Wallet,
    pub index: u64,
    pub pool_fees: JsonValue,
    pub sqrt_min_price: u128,
    pub sqrt_max_price: u128,
    #[serde_as(as = "AsPubkey")]
    pub vault_config_key: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_creator_authority: Pubkey,
    pub activation_type: u8,
    pub collect_fee_mode: u8,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub config: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let config = derive_config(input.index);
    let event_authority = derive_event_authority();

    let accounts = vec![
        AccountMeta::new(input.payer.pubkey(), true),              // payer (writable signer)
        AccountMeta::new(config, false),                           // config (writable, init)
        AccountMeta::new_readonly(input.operator, false),          // operator
        AccountMeta::new(input.signer.pubkey(), true),             // signer (signer)
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),       // system_program
        AccountMeta::new_readonly(event_authority, false),         // event_authority
        AccountMeta::new_readonly(CP_AMM_PROGRAM_ID, false),       // program
    ];

    let mut data = anchor_discriminator(IX_NAME).to_vec();
    data.extend(borsh::to_vec(&input.index)?);
    data.extend(borsh::to_vec(&serde_json::to_string(&input.pool_fees)?)?);
    data.extend(borsh::to_vec(&input.sqrt_min_price)?);
    data.extend(borsh::to_vec(&input.sqrt_max_price)?);
    data.extend(borsh::to_vec(&input.vault_config_key)?);
    data.extend(borsh::to_vec(&input.pool_creator_authority)?);
    data.extend(borsh::to_vec(&input.activation_type)?);
    data.extend(borsh::to_vec(&input.collect_fee_mode)?);

    let instruction = Instruction {
        program_id: CP_AMM_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.payer.pubkey(),
        signers: [input.payer, input.signer].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, config })
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
    /// Required fields: payer, operator, signer, index, pool_fees, sqrt_min_price, sqrt_max_price, vault_config_key, pool_creator_authority, activation_type, collect_fee_mode
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "operator" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "signer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "index" => 1000u64,
            "pool_fees" => serde_json::json!({}),
            "sqrt_min_price" => 0_u128,
            "sqrt_max_price" => 0_u128,
            "vault_config_key" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_creator_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "activation_type" => 0_u8,
            "collect_fee_mode" => 0_u8,
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
            payer: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            operator: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            signer: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").into(),
            index: 1000,
            pool_fees: serde_json::json!({}),
            sqrt_min_price: 1000,
            sqrt_max_price: 1000,
            vault_config_key: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            pool_creator_authority: Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ").pubkey(),
            activation_type: 0,
            collect_fee_mode: 0,
            submit: false,
        };

        let result = run(CommandContext::default(), input).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        let output = result.unwrap();
        println!("{} output: {:?}", NAME, output);
    }
}
