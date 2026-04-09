use super::{PROGRAM_ID, build_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;
use tracing::info;

const NAME: &str = "smart_account_execute_policy_payload_sync";
const DEFINITION: &str =
    flow_lib::node_definition!("smart_account/execute_policy_payload_sync.jsonc");

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
    pub policy_seed: u64,
    pub account_index: u8,
    /// Policy payload type: "spending_limit"
    #[serde(default = "default_policy_type")]
    pub policy_type: String,
    /// Transfer amount in lamports (for spending_limit)
    pub amount: u64,
    /// Transfer destination
    #[serde_as(as = "AsPubkey")]
    pub destination: Pubkey,
    /// Token decimals (default 9 for SOL)
    #[serde(default = "default_decimals")]
    pub decimals: u8,
    /// SPL token mint (omit for native SOL)
    #[serde_as(as = "Option<AsPubkey>")]
    #[serde(default)]
    pub mint: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

fn default_policy_type() -> String {
    "spending_limit".to_string()
}

fn default_decimals() -> u8 {
    9
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (policy, _) = pda::find_policy(&input.settings, input.policy_seed);
    let (smart_account, _) = pda::find_smart_account(&input.settings, input.account_index);

    // executePolicyPayloadSync accounts:
    //   [0] policy (writable) — the policy PDA
    //   [1] program (readonly) — PROGRAM_ID
    // remaining_accounts (for native SOL spending limit):
    //   [2] signer (signer)
    //   [3] source smart account (writable)
    //   [4] destination (writable)
    //   [5] system_program (readonly)
    let mut accounts = vec![
        AccountMeta::new(policy, false),
        AccountMeta::new_readonly(PROGRAM_ID, false),
        // remaining_accounts
        AccountMeta::new_readonly(input.signer.pubkey(), true),
        AccountMeta::new(smart_account, false),
        AccountMeta::new(input.destination, false),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
    ];

    // For SPL token transfers, add mint + token accounts
    if let Some(mint) = input.mint {
        let tp = solana_program::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let ata_program = spl_associated_token_account_interface::program::ID;
        let (source_ata, _) = Pubkey::find_program_address(
            &[smart_account.as_ref(), tp.as_ref(), mint.as_ref()],
            &ata_program,
        );
        let (dest_ata, _) = Pubkey::find_program_address(
            &[input.destination.as_ref(), tp.as_ref(), mint.as_ref()],
            &ata_program,
        );
        accounts.push(AccountMeta::new(source_ata, false));
        accounts.push(AccountMeta::new(dest_ata, false));
        accounts.push(AccountMeta::new_readonly(mint, false));
        accounts.push(AccountMeta::new_readonly(tp, false));
    }

    // SyncTransactionArgs:
    //   account_index: u8
    //   num_signers: u8
    //   payload: SyncPayload (u8 enum)
    //     0 = Transaction
    //     1 = Policy
    let mut args_data = Vec::new();
    args_data.push(input.account_index);
    args_data.push(1u8); // num_signers = 1
    args_data.push(1u8); // SyncPayload discriminant: 1 = Policy

    // PolicyPayload (u8 enum)
    match input.policy_type.as_str() {
        "spending_limit" => {
            args_data.push(2u8); // PolicyPayload::SpendingLimit
            // SpendingLimitPayload { amount: u64, destination: Pubkey, decimals: u8 }
            args_data.extend_from_slice(&input.amount.to_le_bytes());
            args_data.extend_from_slice(input.destination.as_ref());
            args_data.push(input.decimals);
        }
        other => {
            return Err(CommandError::msg(format!(
                "Unsupported policy execution type: {other}. Currently only spending_limit is supported."
            )));
        }
    }

    let instruction = build_instruction("execute_policy_payload_sync", accounts, args_data);

    info!(
        "execute_policy_payload_sync: policy={}, smart_account={}, amount={}, {} accounts",
        policy,
        smart_account,
        input.amount,
        instruction.accounts.len()
    );

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
