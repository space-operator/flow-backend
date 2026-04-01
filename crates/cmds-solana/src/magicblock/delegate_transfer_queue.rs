use super::{DELEGATION_PROGRAM_ID, ETOKEN_PROGRAM_ID, discriminators, pda};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "delegate_transfer_queue";
const DEFINITION: &str = flow_lib::node_definition!("magicblock/delegate_transfer_queue.jsonc");

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
    pub mint: Pubkey,
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
    let buffer = pda::delegation_buffer(&input.queue, &ETOKEN_PROGRAM_ID);
    let delegation_record = pda::delegation_record(&input.queue);
    let delegation_metadata = pda::delegation_metadata(&input.queue);

    let accounts = vec![
        AccountMeta::new_readonly(input.fee_payer.pubkey(), true), // fee_payer (signer, readonly)
        AccountMeta::new(input.queue, false),                      // queue (writable)
        AccountMeta::new_readonly(input.mint, false),              // mint (readonly)
        AccountMeta::new_readonly(ETOKEN_PROGRAM_ID, false),       // owner_program (readonly)
        AccountMeta::new(buffer, false),                           // buffer (writable)
        AccountMeta::new(delegation_record, false),                // delegation_record (writable)
        AccountMeta::new(delegation_metadata, false),              // delegation_metadata (writable)
        AccountMeta::new_readonly(DELEGATION_PROGRAM_ID, false),   // delegation program
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // system_program
    ];

    let data = discriminators::DELEGATE_TRANSFER_QUEUE.to_vec();

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
