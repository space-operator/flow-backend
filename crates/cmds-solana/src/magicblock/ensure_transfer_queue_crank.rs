use super::{ETOKEN_PROGRAM_ID, MAGIC_CONTEXT_ID, MAGIC_PROGRAM_ID, discriminators};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "ensure_transfer_queue_crank";
const DEFINITION: &str = flow_lib::node_definition!("magicblock/ensure_transfer_queue_crank.jsonc");

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
    pub queue: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub magic_fee_vault: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let accounts = vec![
        AccountMeta::new_readonly(input.fee_payer.pubkey(), true), // fee_payer (signer, readonly)
        AccountMeta::new(input.queue, false),                      // queue (writable)
        AccountMeta::new(input.magic_fee_vault, false),            // magic_fee_vault (writable)
        AccountMeta::new(MAGIC_CONTEXT_ID, false),                 // magic_context (writable)
        AccountMeta::new_readonly(MAGIC_PROGRAM_ID, false),        // magic_program (readonly)
    ];

    let data = discriminators::ENSURE_TRANSFER_QUEUE_CRANK.to_vec();

    let instruction = Instruction {
        program_id: ETOKEN_PROGRAM_ID,
        accounts,
        data,
    };

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
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
