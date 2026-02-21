use crate::prelude::*;
use solana_program::program_pack::Pack;
use solana_system_interface::instruction::create_account;

const NAME: &str = "create_token_account";

const DEFINITION: &str = flow_lib::node_definition!("spl_token/create_token_account.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    owner: Pubkey,
    fee_payer: Wallet,
    #[serde(with = "value::pubkey")]
    mint_account: Pubkey,
    token_account: Wallet,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let account = input.token_account.pubkey();

    // Fail fast if account already exists
    if ctx
        .solana_client()
        .get_account_with_commitment(&account, ctx.solana_client().commitment())
        .await?
        .value
        .is_some()
    {
        return Err(CommandError::msg(format!(
            "account already exists: {}",
            account
        )));
    }

    let lamports = ctx
        .solana_client()
        .get_minimum_balance_for_rent_exemption(spl_token_interface::state::Account::LEN)
        .await?;

    let instructions = [
        create_account(
            &input.fee_payer.pubkey(),
            &account,
            lamports,
            spl_token_interface::state::Account::LEN as u64,
            &spl_token_interface::ID,
        ),
        spl_token_interface::instruction::initialize_account3(
            &spl_token_interface::ID,
            &account,
            &input.mint_account,
            &input.owner,
        )?,
    ]
    .into();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.token_account].into(),
        instructions,
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

    #[test]
    fn test_instruction_accounts() {
        let fee_payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let token_account = Pubkey::new_unique();
        let lamports = 2_039_280; // rent-exempt minimum for token account

        let create_ix = create_account(
            &fee_payer,
            &token_account,
            lamports,
            spl_token_interface::state::Account::LEN as u64,
            &spl_token_interface::ID,
        );

        // create_account: [fee_payer (signer, writable), token_account (signer, writable)]
        assert_eq!(create_ix.accounts.len(), 2);
        assert_eq!(create_ix.accounts[0].pubkey, fee_payer);
        assert!(create_ix.accounts[0].is_signer);
        assert!(create_ix.accounts[0].is_writable);
        assert_eq!(create_ix.accounts[1].pubkey, token_account);
        assert!(create_ix.accounts[1].is_signer);
        assert!(create_ix.accounts[1].is_writable);

        let init_ix = spl_token_interface::instruction::initialize_account3(
            &spl_token_interface::ID,
            &token_account,
            &mint,
            &owner,
        )
        .unwrap();

        // initialize_account3: [token_account (writable), mint]
        assert_eq!(init_ix.accounts.len(), 2);
        assert_eq!(init_ix.accounts[0].pubkey, token_account);
        assert!(init_ix.accounts[0].is_writable);
        assert_eq!(init_ix.accounts[1].pubkey, mint);
        assert!(!init_ix.accounts[1].is_writable);
    }
}
