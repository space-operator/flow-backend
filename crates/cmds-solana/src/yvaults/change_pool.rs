use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{YVAULTS_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "change_pool";
const DEFINITION: &str = flow_lib::node_definition!("yvaults/change_pool.jsonc");

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
    pub admin_authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    pub strategy: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub old_position: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub base_vault_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub global_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub new_pool: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub pool_program: Pubkey,
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
        AccountMeta::new(input.admin_authority.pubkey(), true),  // admin_authority (writable signer)
        AccountMeta::new(input.strategy, false),                 // strategy (writable)
        AccountMeta::new(input.old_position, false),             // old_position (writable)
        AccountMeta::new_readonly(input.base_vault_authority, false), // base_vault_authority (readonly)
        AccountMeta::new_readonly(input.global_config, false),   // global_config (readonly)
        AccountMeta::new(input.new_pool, false),                 // new_pool (writable)
        AccountMeta::new_readonly(input.pool_program, false),    // pool_program (readonly)
    ];

    let data = anchor_discriminator(NAME).to_vec();

    let instruction = Instruction {
        program_id: YVAULTS_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.admin_authority].into(),
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
    /// Required fields: fee_payer, admin_authority, strategy, old_position, base_vault_authority, global_config, new_pool, pool_program
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "admin_authority" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "strategy" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "old_position" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "base_vault_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "global_config" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "new_pool" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "pool_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
