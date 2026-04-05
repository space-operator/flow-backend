use super::{build_instruction, pda};
use crate::prelude::*;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_program::instruction::AccountMeta;

const NAME: &str = "smart_account_add_transaction_to_batch";
const DEFINITION: &str = flow_lib::node_definition!("smart_account/add_transaction_to_batch.jsonc");

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
    pub ephemeral_signers: u8,
    /// Borsh-serialized transaction message bytes
    pub transaction_message: Vec<u8>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub transaction: Pubkey,
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
        AccountMeta::new_readonly(proposal, false),
        AccountMeta::new(batch, false),
        AccountMeta::new(transaction, false),
        AccountMeta::new_readonly(input.signer.pubkey(), true),
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
    ];

    // AddTransactionToBatchArgs { ephemeral_signers: u8, transaction_message: bytes }
    let mut args_data = Vec::new();
    args_data.push(input.ephemeral_signers);
    args_data.extend_from_slice(&(input.transaction_message.len() as u32).to_le_bytes());
    args_data.extend_from_slice(&input.transaction_message);

    let instruction = build_instruction("add_transaction_to_batch", accounts, args_data);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.signer.clone()]
            .into_iter()
            .collect(),
        instructions: vec![
            ComputeBudgetInstruction::request_heap_frame(256 * 1024),
            instruction,
        ],
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output {
        signature,
        transaction,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
