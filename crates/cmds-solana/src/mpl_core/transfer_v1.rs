use mpl_core::instructions::TransferV1Builder;
use mpl_core::types::CompressionProof;

use crate::prelude::*;

const NAME: &str = "transfer_v1";
const DEFINITION: &str = flow_lib::node_definition!("mpl_core/transfer_v1.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub new_owner: Pubkey,
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
    let payer = input
        .payer
        .as_ref()
        .map_or_else(|| input.fee_payer.pubkey(), |p| p.pubkey());
    let mut builder = TransferV1Builder::new();
    builder
        .asset(input.asset)
        .payer(payer)
        .new_owner(input.new_owner);
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

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
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
