use crate::prelude::*;
use mpl_core::instructions::ApprovePluginAuthorityV1Builder;
use mpl_core::types::{PluginAuthority, PluginType};

const NAME: &str = "approve_plugin_authority_v1";
const DEFINITION: &str = flow_lib::node_definition!("mpl_core/approve_plugin_authority_v1.jsonc");

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
    pub asset: Pubkey,
    pub payer: Option<Wallet>,
    pub plugin_type: PluginType,
    pub new_authority: PluginAuthority,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let payer = input.payer.as_ref().map_or_else(|| input.fee_payer.pubkey(), |p| p.pubkey());
    // Build instruction using builder pattern
    let instruction = ApprovePluginAuthorityV1Builder::new()
        .asset(input.asset)
        .payer(payer)
        .plugin_type(input.plugin_type)
        .new_authority(input.new_authority)
        .instruction();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into_iter().chain(input.payer)
            .collect(),
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
    /// Required fields: fee_payer, asset, payer, plugin_type, new_authority
    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "fee_payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "asset" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
            "payer" => "4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ",
            "plugin_type" => serde_json::json!("FreezeDelegate"),
            "new_authority" => serde_json::json!("Owner"),
            "submit" => false,
        };

        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
