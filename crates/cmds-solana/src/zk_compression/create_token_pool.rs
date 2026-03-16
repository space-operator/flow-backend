use crate::prelude::*;
use super::{to_pubkey_v2, to_instruction_v3};
use light_compressed_token_sdk::spl_interface::CreateSplInterfacePda;
use light_compressed_token_sdk::constants::SPL_TOKEN_PROGRAM_ID;

const NAME: &str = "create_token_pool";

const DEFINITION: &str = flow_lib::node_definition!("zk_compression/create_token_pool.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    fee_payer: Wallet,
    #[serde(with = "value::pubkey")]
    mint: Pubkey,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let fee_payer_v2 = to_pubkey_v2(&input.fee_payer.pubkey());
    let mint_v2 = to_pubkey_v2(&input.mint);
    let token_program_v2 = SPL_TOKEN_PROGRAM_ID;

    // CreateSplInterfacePda::new derives the PDA and builds the instruction
    let pda = CreateSplInterfacePda::new(fee_payer_v2, mint_v2, token_program_v2, false);
    let ix_v2 = pda.instruction();
    let instruction = to_instruction_v3(ix_v2);

    let ins = if input.submit {
        Instructions {
            lookup_tables: None,
            fee_payer: input.fee_payer.pubkey(),
            signers: [input.fee_payer].into(),
            instructions: [instruction].into(),
        }
    } else {
        <_>::default()
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
