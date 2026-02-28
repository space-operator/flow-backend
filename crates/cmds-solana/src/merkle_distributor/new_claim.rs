use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{MERKLE_DISTRIBUTOR_PROGRAM_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "new_claim";
const DEFINITION: &str = flow_lib::node_definition!("merkle_distributor/new_claim.jsonc");

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
    pub distributor: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub from: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub to: Pubkey,
    pub claimant: Wallet,
    pub amount_unlocked: u64,
    pub amount_locked: u64,
    pub proof: JsonValue,
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
        AccountMeta::new(input.distributor, false),            // distributor (writable)
        AccountMeta::new(input.from, false),                   // from (writable - token vault)
        AccountMeta::new(input.to, false),                     // to (writable - destination)
        AccountMeta::new(input.claimant.pubkey(), true),       // claimant (writable signer)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),    // token_program
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),   // system_program
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.amount_unlocked)?);
    data.extend(borsh::to_vec(&input.amount_locked)?);
    let proof_bytes: Vec<[u8; 32]> = serde_json::from_value(input.proof)?;
    data.extend(borsh::to_vec(&proof_bytes)?);

    let instruction = Instruction {
        program_id: MERKLE_DISTRIBUTOR_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.claimant].into(),
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
    /// Required fields: fee_payer, distributor, from, to, claimant, amount_unlocked, amount_locked, proof
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "distributor" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "from" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "to" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "claimant" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "amount_unlocked" => 1000u64,
            "amount_locked" => 1000u64,
            "proof" => serde_json::json!({}),
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
