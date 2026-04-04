use super::{PROGRAM_ID, build_instruction, pda};
use crate::prelude::*;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_program::instruction::AccountMeta;

const NAME: &str = "smart_account_create_transaction";
const DEFINITION: &str = flow_lib::node_definition!("smart_account/create_transaction.jsonc");

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
    pub transaction_index: u64,
    pub account_index: u8,
    #[serde(default)]
    pub ephemeral_signers: u8,
    /// Borsh-serialized transaction message bytes
    pub transaction_message: Vec<u8>,
    #[serde(default)]
    pub memo: Option<String>,
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
    let (transaction, _) = pda::find_transaction(&input.settings, input.transaction_index);

    let accounts = vec![
        AccountMeta::new(input.settings, false),
        AccountMeta::new(transaction, false),
        AccountMeta::new_readonly(input.creator.pubkey(), true),
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(PROGRAM_ID, false),
    ];

    // CreateTransactionArgs is a Borsh ENUM:
    //   enum CreateTransactionArgs {
    //     TransactionPayload(TransactionPayload),  // variant 0
    //     PolicyPayload { payload: PolicyPayload }, // variant 1
    //   }
    // Must prepend variant discriminator byte (0 = TransactionPayload).
    let mut args_data = Vec::new();
    args_data.push(0u8); // enum variant 0: TransactionPayload
    args_data.push(input.account_index);
    args_data.push(input.ephemeral_signers);
    // transaction_message as bytes (Vec<u8> borsh: u32 len + data)
    args_data.extend_from_slice(&(input.transaction_message.len() as u32).to_le_bytes());
    args_data.extend_from_slice(&input.transaction_message);
    match &input.memo {
        Some(s) => {
            args_data.push(1);
            args_data.extend_from_slice(&(s.len() as u32).to_le_bytes());
            args_data.extend_from_slice(s.as_bytes());
        }
        None => args_data.push(0),
    }

    let instruction = build_instruction("create_transaction", accounts, args_data);

    // The Squads smart-account program requires more than the default 32KB heap
    // for deserializing Settings and VaultTransactionMessage structs.
    let heap_ix = ComputeBudgetInstruction::request_heap_frame(256 * 1024);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.creator.clone()]
            .into_iter()
            .collect(),
        instructions: vec![heap_ix, instruction],
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
