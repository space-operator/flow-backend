use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};
use super::{YVAULTS_PROGRAM_ID, anchor_discriminator};

const NAME: &str = "yvaults_update_shares_metadata";
const IX_NAME: &str = "update_shares_metadata";
const DEFINITION: &str = flow_lib::node_definition!("yvaults/update_shares_metadata.jsonc");

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
    pub global_config: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub shares_mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub shares_metadata: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub shares_mint_authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub metadata_program: Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
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
        AccountMeta::new_readonly(input.global_config, false),   // global_config (readonly)
        AccountMeta::new(input.shares_mint, false),              // shares_mint (writable)
        AccountMeta::new(input.shares_metadata, false),          // shares_metadata (writable)
        AccountMeta::new_readonly(input.shares_mint_authority, false), // shares_mint_authority (readonly)
        AccountMeta::new_readonly(input.metadata_program, false), // metadata_program (readonly)
    ];

    let mut data = anchor_discriminator(IX_NAME).to_vec();
    data.extend(borsh::to_vec(&input.name)?);
    data.extend(borsh::to_vec(&input.symbol)?);
    data.extend(borsh::to_vec(&input.uri)?);

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
    /// Required fields: fee_payer, admin_authority, strategy, global_config, shares_mint, shares_metadata, shares_mint_authority, metadata_program, name, symbol, uri
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "admin_authority" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "strategy" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "global_config" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "shares_mint" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "shares_metadata" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "shares_mint_authority" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "metadata_program" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "name" => "test_name",
            "symbol" => "test_symbol",
            "uri" => "test_uri",
            "submit" => false,
        };
        
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
