use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{KFARMS_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "update_farm_config";
const DEFINITION: &str = flow_lib::node_definition!("kfarms/update_farm_config.jsonc");

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
    pub farm_admin: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub farm_state: Pubkey,
    pub mode: u16,
    pub data: JsonValue,
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
        AccountMeta::new(input.farm_admin.pubkey(), true), // farmAdmin (writable signer)
        AccountMeta::new(input.farm_state, false),         // farmState (writable)
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.mode)?);
    // data is [u8; 32] on-chain, serialize the JSON bytes padded to 32
    let json_bytes = serde_json::to_vec(&input.data)?;
    let mut data_arr = [0u8; 32];
    let len = json_bytes.len().min(32);
    data_arr[..len].copy_from_slice(&json_bytes[..len]);
    data.extend(borsh::to_vec(&data_arr)?);

    let instruction = Instruction {
        program_id: KFARMS_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.farm_admin].into(),
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
    /// Required fields: fee_payer, farm_admin, farm_state, mode, data
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "farm_admin" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "farm_state" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "mode" => 0_u16,
            "data" => serde_json::json!({}),
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
