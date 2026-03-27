use super::{build_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "smart_account_extend_transaction_buffer";
const DEFINITION: &str =
    flow_lib::node_definition!("smart_account/extend_transaction_buffer.jsonc");

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
    pub settings: Pubkey,
    pub creator: Wallet,
    pub buffer_index: u8,
    pub buffer: Vec<u8>,
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
    let (transaction_buffer, _) =
        pda::find_transaction_buffer(&input.settings, &input.creator.pubkey(), input.buffer_index);

    let accounts = vec![
        AccountMeta::new_readonly(input.settings, false),
        AccountMeta::new(transaction_buffer, false),
        AccountMeta::new_readonly(input.creator.pubkey(), true),
    ];

    // ExtendTransactionBufferArgs { buffer: bytes }
    let mut args_data = Vec::new();
    args_data.extend_from_slice(&(input.buffer.len() as u32).to_le_bytes());
    args_data.extend_from_slice(&input.buffer);

    let instruction = build_instruction("extend_transaction_buffer", accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.creator.clone()]
            .into_iter()
            .collect(),
        instructions: vec![instruction],
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
