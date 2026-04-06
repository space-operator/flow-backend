use super::{build_instruction, pda};
use crate::prelude::*;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_program::instruction::AccountMeta;
use tracing::info;

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

    // Read the batch transaction account to get inner instruction accounts.
    // Retry since the account may have been created in the same flow run.
    let tx_data = {
        let client = ctx.solana_client();
        let mut data = None;
        for attempt in 0..10u32 {
            match client.get_account_data(&transaction).await {
                Ok(d) => {
                    data = Some(d);
                    break;
                }
                Err(_) if attempt < 9 => {
                    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                }
                Err(e) => {
                    return Err(CommandError::msg(format!(
                        "Failed to read batch transaction account after retries: {e}"
                    )));
                }
            }
        }
        data.ok_or_else(|| CommandError::msg("Batch transaction account not found after retries"))?
    };

    // Parse the stored SmartAccountTransactionMessage (Borsh format).
    // BatchTransaction layout (from IDL):
    //   disc: 8 + bump: u8 (1) + rentCollector: Pubkey (32) = 41
    //   ephemeralSignerBumps: Vec<u8> (Borsh: u32 + N)
    //   message: SmartAccountTransactionMessage (Borsh)
    let mut offset = 8 + 1 + 32; // 41
    if offset + 4 > tx_data.len() {
        return Err(CommandError::msg(
            "BatchTransaction data too short for ephemeralSignerBumps",
        ));
    }
    let bumps_len =
        u32::from_le_bytes(tx_data[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4 + bumps_len;

    // SmartAccountTransactionMessage header (Borsh)
    if offset + 3 > tx_data.len() {
        return Err(CommandError::msg(
            "BatchTransaction data too short for message header",
        ));
    }
    let _num_signers = tx_data[offset] as usize;
    let num_writable_signers = tx_data[offset + 1] as usize;
    let num_writable_non_signers = tx_data[offset + 2] as usize;
    offset += 3;

    // account_keys: Vec<Pubkey> (Borsh: u32 + N*32)
    if offset + 4 > tx_data.len() {
        return Err(CommandError::msg(
            "BatchTransaction data too short for account_keys length",
        ));
    }
    let num_keys =
        u32::from_le_bytes(tx_data[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;

    let mut remaining_accounts = Vec::with_capacity(num_keys);
    for i in 0..num_keys {
        if offset + 32 > tx_data.len() {
            return Err(CommandError::msg(
                "BatchTransaction data too short for account key",
            ));
        }
        let key = Pubkey::try_from(&tx_data[offset..offset + 32])
            .map_err(|_| CommandError::msg("Invalid account key"))?;
        offset += 32;

        // Writable flags from the message header layout
        let is_writable = i < num_writable_signers
            || (i >= _num_signers && i < _num_signers + num_writable_non_signers);

        // Never mark as signer — program handles via invoke_signed
        remaining_accounts.push(AccountMeta {
            pubkey: key,
            is_signer: false,
            is_writable,
        });
    }

    // Build accounts: 5 named accounts + remaining_accounts
    // execute_batch_transaction Anchor struct:
    //   0: settings (writable)
    //   1: member/signer (signer)
    //   2: proposal (writable)
    //   3: batch (writable)
    //   4: transaction (readonly)
    // NO program field — unlike execute_transaction
    let mut accounts = vec![
        AccountMeta::new(input.settings, false),
        AccountMeta::new_readonly(input.signer.pubkey(), true),
        AccountMeta::new(proposal, false),
        AccountMeta::new(batch, false),
        AccountMeta::new_readonly(transaction, false),
    ];
    accounts.extend(remaining_accounts.iter().cloned());

    let instruction = build_instruction("execute_batch_transaction", accounts, vec![]);

    info!(
        "execute_batch_transaction: {} total accounts, {} remaining",
        instruction.accounts.len(),
        remaining_accounts.len()
    );
    for (idx, acc) in instruction.accounts.iter().enumerate() {
        info!(
            "  account[{}]: {} (signer={}, writable={})",
            idx, acc.pubkey, acc.is_signer, acc.is_writable
        );
    }

    let heap_ix = ComputeBudgetInstruction::request_heap_frame(256 * 1024);

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.signer.clone()]
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
