use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{KFARMS_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "kfarms_update_global_config";
const IX_NAME: &str = "update_global_config";
const DEFINITION: &str = flow_lib::node_definition!("kfarms/update_global_config.jsonc");

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
    pub global_admin: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub global_config: Pubkey,
    pub mode: u8,
    pub value: JsonValue,
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
        AccountMeta::new_readonly(input.global_admin.pubkey(), true), // globalAdmin (signer)
        AccountMeta::new(input.global_config, false),                 // globalConfig (writable)
    ];

    let mut data = anchor_discriminator(IX_NAME).to_vec();
    data.extend(borsh::to_vec(&input.mode)?);
    // value is [u8; 32] on-chain, serialize the JSON bytes padded to 32
    let json_bytes = serde_json::to_vec(&input.value)?;
    let mut value_arr = [0u8; 32];
    let len = json_bytes.len().min(32);
    value_arr[..len].copy_from_slice(&json_bytes[..len]);
    data.extend(borsh::to_vec(&value_arr)?);

    let instruction = Instruction {
        program_id: KFARMS_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.global_admin].into(),
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
    /// Required fields: fee_payer, global_admin, global_config, mode, value
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "global_admin" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "global_config" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "mode" => 0_u8,
            "value" => serde_json::json!({}),
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
