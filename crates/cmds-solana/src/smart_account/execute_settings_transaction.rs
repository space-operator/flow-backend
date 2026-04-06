use super::{PROGRAM_ID, build_instruction, pda};
use crate::prelude::*;
use solana_program::instruction::AccountMeta;

const NAME: &str = "smart_account_execute_settings_transaction";
const DEFINITION: &str =
    flow_lib::node_definition!("smart_account/execute_settings_transaction.jsonc");

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
    pub transaction_index: u64,
    /// Optional policy PDAs to pass as remaining_accounts.
    /// Required when the settings transaction creates/updates/removes policies.
    #[serde_as(as = "Option<Vec<AsPubkey>>")]
    #[serde(default)]
    pub policy_pdas: Option<Vec<Pubkey>>,
    /// Optional rent payer for policy account creation.
    #[serde_as(as = "Option<AsPubkey>")]
    #[serde(default)]
    pub rent_payer: Option<Pubkey>,
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
    let (proposal, _) = pda::find_proposal(&input.settings, input.transaction_index);
    let (transaction, _) = pda::find_transaction(&input.settings, input.transaction_index);

    let mut accounts = vec![
        AccountMeta::new(input.settings, false),
        AccountMeta::new_readonly(input.signer.pubkey(), true),
        AccountMeta::new(proposal, false),
        AccountMeta::new_readonly(transaction, false),
        AccountMeta::new(input.fee_payer.pubkey(), true),
        AccountMeta::new_readonly(solana_system_interface::program::ID, false),
        AccountMeta::new_readonly(PROGRAM_ID, false),
    ];

    // remaining_accounts for policy operations:
    // PolicyCreate needs: [policy_pda (writable), rent_payer (writable, signer)]
    if let Some(pdas) = &input.policy_pdas {
        for pda in pdas {
            accounts.push(AccountMeta::new(*pda, false));
        }
        // rent_payer for policy account creation (defaults to fee_payer)
        let rent_payer = input.rent_payer.unwrap_or(input.fee_payer.pubkey());
        accounts.push(AccountMeta::new(rent_payer, true));
    }

    tracing::info!(
        "execute_settings_transaction: {} accounts, policy_pdas={:?}",
        accounts.len(),
        input.policy_pdas,
    );

    let instruction = build_instruction("execute_settings_transaction", accounts, vec![]);

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
