use mpl_core::{
    instructions::CreateV2Builder,
    types::{ExternalPluginAdapterInitInfo, PluginAuthorityPair},
};

use crate::prelude::*;

// Command Name
const NAME: &str = "create_core_v2";

const DEFINITION: &str = flow_lib::node_definition!("mpl_core/mpl_core_create_asset.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub fee_payer: Wallet,
    pub asset: Wallet,
    pub authority: Option<Wallet>,
    pub name: String,
    pub uri: String,
    pub plugins: Option<Vec<PluginAuthorityPair>>,
    pub external_plugin_adapters: Option<Vec<ExternalPluginAdapterInitInfo>>,
    pub verified_creator: Option<Wallet>,
    #[serde(default, with = "value::pubkey::opt")]
    pub collection: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut builder = CreateV2Builder::new();

    builder
        .asset(input.asset.pubkey())
        .payer(input.fee_payer.pubkey())
        .name(input.name)
        .uri(input.uri);

    if let Some(ref authority) = input.authority {
        builder.authority(Some(authority.pubkey()));
    }
    if let Some(collection) = input.collection {
        builder.collection(Some(collection));
    }
    if let Some(plugins) = input.plugins {
        builder.plugins(plugins);
    }
    if let Some(external_plugin_adapters) = input.external_plugin_adapters {
        builder.external_plugin_adapters(external_plugin_adapters);
    }

    let ins = builder.instruction();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.asset]
            .into_iter()
            .chain(input.authority)
            .chain(input.verified_creator)
            .collect(),
        instructions: [ins].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
