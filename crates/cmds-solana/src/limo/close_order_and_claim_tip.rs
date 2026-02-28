use super::derive_ata;
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{LIMO_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "close_order_and_claim_tip";
const DEFINITION: &str = flow_lib::node_definition!("limo/close_order_and_claim_tip.jsonc");

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
    pub maker: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub order: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub global_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pda_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub input_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub output_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub input_vault: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub input_token_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub event_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub program: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub maker_input_ata: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let maker_input_ata = derive_ata(&input.maker.pubkey(), &input.input_mint, &input.input_token_program);

    let accounts = vec![
        AccountMeta::new(input.maker.pubkey(), true),                // maker (writable signer)
        AccountMeta::new(input.order, false),                        // order (writable)
        AccountMeta::new_readonly(input.global_config, false),       // global_config
        AccountMeta::new_readonly(input.pda_authority, false),       // pda_authority
        AccountMeta::new_readonly(input.input_mint, false),          // input_mint
        AccountMeta::new_readonly(input.output_mint, false),         // output_mint
        AccountMeta::new(maker_input_ata, false),              // maker_input_ata (writable)
        AccountMeta::new(input.input_vault, false),                  // input_vault (writable)
        AccountMeta::new_readonly(input.input_token_program, false), // input_token_program
        AccountMeta::new_readonly(input.event_authority, false),     // event_authority
        AccountMeta::new_readonly(input.program, false),             // program
    ];

    let data = anchor_discriminator(NAME).to_vec();

    let instruction = Instruction {
        program_id: LIMO_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.maker].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, maker_input_ata })
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
    /// Required fields: fee_payer, maker, order, global_config, pda_authority, input_mint, output_mint, input_vault, input_token_program, event_authority, program
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "maker" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "order" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "global_config" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pda_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "input_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "output_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "input_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "input_token_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "event_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
