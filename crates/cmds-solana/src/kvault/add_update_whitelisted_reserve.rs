use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{KVAULT_PROGRAM_ID, SYSTEM_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "add_update_whitelisted_reserve";
const DEFINITION: &str = flow_lib::node_definition!("kvault/add_update_whitelisted_reserve.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub reserve: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub reserve_whitelist_entry: Pubkey,
    pub update: serde_json::Value,
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
        AccountMeta::new(input.global_admin.pubkey(), true),         // global_admin (writable signer)
        AccountMeta::new_readonly(input.global_config, false),       // global_config
        AccountMeta::new_readonly(input.reserve, false),             // reserve
        AccountMeta::new(input.reserve_whitelist_entry, false),      // reserve_whitelist_entry (writable)
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),         // system_program
    ];

    let mut data = anchor_discriminator("add_update_whitelisted_reserve").to_vec();
    data.extend(serde_json::to_vec(&input.update)?);

    let instruction = Instruction {
        program_id: KVAULT_PROGRAM_ID,
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
    /// Required fields: fee_payer, global_admin, global_config, reserve, reserve_whitelist_entry, update
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "global_admin" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "global_config" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reserve" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "reserve_whitelist_entry" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "update" => serde_json::json!({}),
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
