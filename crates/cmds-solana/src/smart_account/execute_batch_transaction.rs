use super::{build_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "smart_account_execute_batch_transaction";
const DEFINITION: &str =
    flow_lib::node_definition!("smart_account/execute_batch_transaction.jsonc");

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
    pub signer: Wallet,
    pub batch_index: u64,
    pub batch_transaction_index: u32,
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
    let (batch, _) = pda::find_transaction(&input.settings, input.batch_index);
    let (proposal, _) = pda::find_proposal(&input.settings, input.batch_index);
    let (transaction, _) = pda::find_batch_transaction(
        &input.settings,
        input.batch_index,
        input.batch_transaction_index,
    );

    let accounts = vec![
        AccountMeta::new_readonly(input.settings, false),
        AccountMeta::new_readonly(input.signer.pubkey(), true),
        AccountMeta::new(proposal, false),
        AccountMeta::new(batch, false),
        AccountMeta::new_readonly(transaction, false),
    ];

    let instruction = build_instruction("execute_batch_transaction", accounts, vec![]);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.signer.clone()]
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
