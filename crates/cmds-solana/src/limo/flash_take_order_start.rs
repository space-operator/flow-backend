use super::derive_ata;
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{LIMO_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "flash_take_order_start";
const DEFINITION: &str = flow_lib::node_definition!("limo/flash_take_order_start.jsonc");

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
    pub taker: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub maker: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub global_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pda_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub order: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub input_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub output_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub input_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub express_relay: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub express_relay_metadata: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub config_router: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub input_token_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub output_token_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub event_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub program: Pubkey,
    pub input_amount: u64,
    pub min_output_amount: u64,
    pub tip_amount_permissionless_taking: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub taker_input_ata: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub taker_output_ata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let taker_input_ata = derive_ata(&input.taker.pubkey(), &input.input_mint, &input.input_token_program);
    let taker_output_ata = derive_ata(&input.taker.pubkey(), &input.output_mint, &input.output_token_program);

    let accounts = vec![
        AccountMeta::new(input.taker.pubkey(), true),                    // taker (writable signer)
        AccountMeta::new_readonly(input.maker, false),                   // maker
        AccountMeta::new_readonly(input.global_config, false),           // global_config
        AccountMeta::new_readonly(input.pda_authority, false),           // pda_authority
        AccountMeta::new(input.order, false),                            // order (writable)
        AccountMeta::new_readonly(input.input_mint, false),              // input_mint
        AccountMeta::new_readonly(input.output_mint, false),             // output_mint
        AccountMeta::new(input.input_vault, false),                      // input_vault (writable)
        AccountMeta::new(taker_input_ata, false),                  // taker_input_ata (writable)
        AccountMeta::new(taker_output_ata, false),                 // taker_output_ata (writable)
        AccountMeta::new_readonly(input.express_relay, false),           // express_relay
        AccountMeta::new_readonly(input.express_relay_metadata, false),  // express_relay_metadata
        AccountMeta::new_readonly(input.config_router, false),           // config_router
        AccountMeta::new_readonly(input.input_token_program, false),     // input_token_program
        AccountMeta::new_readonly(input.output_token_program, false),    // output_token_program
        AccountMeta::new_readonly(input.event_authority, false),         // event_authority
        AccountMeta::new_readonly(input.program, false),                 // program
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.input_amount)?);
    data.extend(borsh::to_vec(&input.min_output_amount)?);
    data.extend(borsh::to_vec(&input.tip_amount_permissionless_taking)?);

    let instruction = Instruction {
        program_id: LIMO_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.taker].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, taker_input_ata, taker_output_ata })
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
    /// Required fields: fee_payer, taker, maker, global_config, pda_authority, order, input_mint, output_mint, input_vault, express_relay, express_relay_metadata, config_router, input_token_program, output_token_program, event_authority, program, input_amount, min_output_amount, tip_amount_permissionless_taking
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "taker" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "maker" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "global_config" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pda_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "order" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "input_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "output_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "input_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "express_relay" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "express_relay_metadata" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "config_router" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "input_token_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "output_token_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "event_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "input_amount" => 1000u64,
            "min_output_amount" => 1000u64,
            "tip_amount_permissionless_taking" => 1000u64,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
