use mpl_core::instructions::UpdateCollectionV1Builder;

use crate::prelude::*;

const NAME: &str = "update_collection_v1";
const DEFINITION: &str = flow_lib::node_definition!("mpl_core/update_collection_v1.jsonc");

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
    pub collection: Pubkey,
    pub payer: Option<Wallet>,
    pub new_name: Option<String>,
    pub new_uri: Option<String>,
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
    let mut builder = UpdateCollectionV1Builder::new();
    builder
        .collection(input.collection)
        .payer(payer);
    if let Some(new_name) = input.new_name {
        builder.new_name(new_name);
    }
    if let Some(new_uri) = input.new_uri {
        builder.new_uri(new_uri);
    }
    let instruction = builder.instruction();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into_iter().chain(input.payer).collect(),
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
}
