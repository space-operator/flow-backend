use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{MERKLE_DISTRIBUTOR_PROGRAM_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "new_distributor";
const DEFINITION: &str = flow_lib::node_definition!("merkle_distributor/new_distributor.jsonc");

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
    pub base: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub clawback_receiver: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub token_vault: Pubkey,
    pub admin: Wallet,
    pub version: u64,
    pub root: JsonValue,
    pub max_total_claim: u64,
    pub max_num_nodes: u64,
    pub start_vesting_ts: i64,
    pub end_vesting_ts: i64,
    pub clawback_start_ts: i64,
    pub enable_slot: u64,
    pub closable: bool,
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
        AccountMeta::new(input.base.pubkey(), true),              // base (writable signer)
        AccountMeta::new_readonly(input.clawback_receiver, false), // clawback_receiver
        AccountMeta::new_readonly(input.mint, false),              // mint
        AccountMeta::new(input.token_vault, false),                // token_vault (writable)
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),        // token_program
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),       // system_program
        AccountMeta::new(input.admin.pubkey(), true),              // admin (writable signer)
    ];

    let mut data = anchor_discriminator(NAME).to_vec();
    data.extend(borsh::to_vec(&input.version)?);
    let root: [u8; 32] = serde_json::from_value(input.root)?;
    data.extend(borsh::to_vec(&root)?);
    data.extend(borsh::to_vec(&input.max_total_claim)?);
    data.extend(borsh::to_vec(&input.max_num_nodes)?);
    data.extend(borsh::to_vec(&input.start_vesting_ts)?);
    data.extend(borsh::to_vec(&input.end_vesting_ts)?);
    data.extend(borsh::to_vec(&input.clawback_start_ts)?);
    data.extend(borsh::to_vec(&input.enable_slot)?);
    data.extend(borsh::to_vec(&input.closable)?);

    let instruction = Instruction {
        program_id: MERKLE_DISTRIBUTOR_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.base, input.admin].into(),
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
    /// Required fields: fee_payer, base, clawback_receiver, mint, token_vault, admin, version, root, max_total_claim, max_num_nodes, start_vesting_ts, end_vesting_ts, clawback_start_ts, enable_slot, closable
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "base" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "clawback_receiver" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "token_vault" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "admin" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "version" => 1000u64,
            "root" => serde_json::json!({}),
            "max_total_claim" => 1000u64,
            "max_num_nodes" => 1000u64,
            "start_vesting_ts" => 1000u64,
            "end_vesting_ts" => 1000u64,
            "clawback_start_ts" => 1000u64,
            "enable_slot" => 1000u64,
            "closable" => false,
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
