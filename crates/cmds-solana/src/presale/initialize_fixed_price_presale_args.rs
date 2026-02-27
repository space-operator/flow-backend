use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{PRESALE_PROGRAM_ID, derive_event_authority, discriminators};

const NAME: &str = "initialize_fixed_price_presale_args";
const DEFINITION: &str = flow_lib::node_definition!("presale/initialize_fixed_price_presale_args.jsonc");

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
    pub fixed_price_presale_params: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub owner: Pubkey,
    pub payer: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub system_program: Pubkey,
    /// The presale pubkey (arg for the instruction)
    #[serde_as(as = "AsPubkey")]
    pub presale: Pubkey,
    pub disable_withdraw: u8,
    pub q_price: u128,
    #[serde(default)]
    pub padding1: [u64; 8],
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
        AccountMeta::new(input.fixed_price_presale_params, false),  // fixed_price_presale_params (writable, PDA)
        AccountMeta::new_readonly(input.owner, false),              // owner (readonly)
        AccountMeta::new(input.payer.pubkey(), true),               // payer (writable, signer)
        AccountMeta::new_readonly(input.system_program, false),     // system_program (readonly)
        AccountMeta::new_readonly(event_authority, false),          // event_authority (PDA)
        AccountMeta::new_readonly(PRESALE_PROGRAM_ID, false),       // program
    ];

    // Manually serialize InitializeFixedPricePresaleExtraArgs to avoid borsh version conflict
    // (Pubkey implements BorshSerialize from borsh 1.x but crate uses borsh 0.10)
    let mut data = discriminators::INITIALIZE_FIXED_PRICE_PRESALE_ARGS.to_vec();
    data.extend_from_slice(input.presale.as_ref());        // presale: Pubkey (32 bytes)
    data.push(input.disable_withdraw);                     // disable_withdraw: u8
    data.extend_from_slice(&input.q_price.to_le_bytes());  // q_price: u128 (16 bytes)
    for v in &input.padding1 {                             // padding1: [u64; 8]
        data.extend_from_slice(&v.to_le_bytes());
    }

    let instruction = Instruction {
        program_id: PRESALE_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.payer].into(),
        instructions: [instruction].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
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

    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "fixed_price_presale_params" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "owner" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "system_program" => "11111111111111111111111111111111",
            "presale" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "disable_withdraw" => 0u64,
            "q_price" => 1000000u64,
            "submit" => false,
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
