use mpl_core::instructions::BurnCollectionV1Builder;
use mpl_core::types::CompressionProof;

use crate::prelude::*;

const NAME: &str = "burn_collection_v1";
const DEFINITION: &str = flow_lib::node_definition!("mpl_core/burn_collection_v1.jsonc");

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
    pub compression_proof: Option<CompressionProof>,
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
    let mut builder = BurnCollectionV1Builder::new();
    builder
        .collection(input.collection)
        .payer(payer);
    if let Some(proof) = input.compression_proof {
        builder.compression_proof(proof);
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
