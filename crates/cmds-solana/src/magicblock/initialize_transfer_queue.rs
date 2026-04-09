use super::{ETOKEN_PROGRAM_ID, PERMISSION_PROGRAM_ID, discriminators, pda};
use crate::prelude::*;
use solana_program::instruction::{AccountMeta, Instruction};

const NAME: &str = "initialize_transfer_queue";
const DEFINITION: &str = flow_lib::node_definition!("magicblock/initialize_transfer_queue.jsonc");

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
    pub mint: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub validator: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub queue_permission: Pubkey,
    pub requested_items: u32,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub queue: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let queue = pda::transfer_queue(&input.mint, &input.validator);

    let accounts = vec![
        AccountMeta::new(input.fee_payer.pubkey(), true), // fee_payer (writable, signer)
        AccountMeta::new(queue, false),                   // queue PDA (writable)
        AccountMeta::new(input.queue_permission, false),  // queue_permission (writable)
        AccountMeta::new_readonly(input.mint, false),     // mint (readonly)
        AccountMeta::new_readonly(input.validator, false), // validator (readonly)
        AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // system_program
        AccountMeta::new_readonly(PERMISSION_PROGRAM_ID, false), // permission program
    ];

    let mut data = discriminators::INITIALIZE_TRANSFER_QUEUE.to_vec();
    data.extend(input.requested_items.to_le_bytes());

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

    Ok(Output { signature, queue })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
