use super::{PROGRAM_ID, build_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "smart_account_use_spending_limit";
const DEFINITION: &str = flow_lib::node_definition!("smart_account/use_spending_limit.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub spending_limit: Pubkey,
    pub account_index: u8,
    #[serde_as(as = "AsPubkey")]
    pub destination: Pubkey,
    #[serde_as(as = "Option<AsPubkey>")]
    #[serde(default)]
    pub mint: Option<Pubkey>,
    #[serde_as(as = "Option<AsPubkey>")]
    #[serde(default)]
    pub token_program: Option<Pubkey>,
    pub amount: u64,
    pub decimals: u8,
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
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (smart_account, _) = pda::find_smart_account(&input.settings, input.account_index);

    let mut accounts = vec![
        AccountMeta::new_readonly(input.settings, false),
        AccountMeta::new_readonly(input.signer.pubkey(), true),
        AccountMeta::new(input.spending_limit, false),
        AccountMeta::new(smart_account, false),
        AccountMeta::new(input.destination, false),
        // systemProgram (optional)
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
    ];

    // Optional SPL token accounts
    if let Some(mint) = input.mint {
        accounts.push(AccountMeta::new_readonly(mint, false));
        let tp = input.token_program.unwrap_or(solana_program::pubkey!(
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        ));
        let ata_program = spl_associated_token_account_interface::program::ID;
        let (vault_ata, _) = Pubkey::find_program_address(
            &[smart_account.as_ref(), tp.as_ref(), mint.as_ref()],
            &ata_program,
        );
        let (dest_ata, _) = Pubkey::find_program_address(
            &[input.destination.as_ref(), tp.as_ref(), mint.as_ref()],
            &ata_program,
        );
        accounts.push(AccountMeta::new(vault_ata, false));
        accounts.push(AccountMeta::new(dest_ata, false));
        accounts.push(AccountMeta::new_readonly(tp, false));
    }

    accounts.push(AccountMeta::new_readonly(PROGRAM_ID, false));

    // UseSpendingLimitArgs { amount: u64, decimals: u8, memo: Option<String> }
    let mut args_data = Vec::new();
    args_data.extend_from_slice(&input.amount.to_le_bytes());
    args_data.push(input.decimals);
    match &input.memo {
        Some(s) => {
            args_data.push(1);
            args_data.extend_from_slice(&(s.len() as u32).to_le_bytes());
            args_data.extend_from_slice(s.as_bytes());
        }
        None => args_data.push(0),
    }

    let instruction = build_instruction("use_spending_limit", accounts, args_data);

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
