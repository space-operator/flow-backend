use super::derive_ata;
use crate::prelude::*;

const NAME: &str = "burn_checked_t22";
const DEFINITION: &str = flow_lib::node_definition!("spl_token_2022/burn_checked.jsonc");

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
    pub authority: Wallet,
    /// Optional source-account owner for delegate / clawback burns.
    /// When omitted, the account to burn from is derived from `authority.pubkey()`
    /// (normal owner-initiated burn). When set, the account is derived from this
    /// pubkey and `authority` acts as a signing delegate (PermanentDelegate
    /// revocation, or an account previously approved via spl_token::approve).
    #[serde(default)]
    #[serde_as(as = "Option<AsPubkey>")]
    pub account_owner: Option<Pubkey>,
    pub amount: u64,
    pub decimals: u8,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub account: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let account_owner = input
        .account_owner
        .unwrap_or_else(|| input.authority.pubkey());
    let account = derive_ata(&account_owner, &input.mint);

    let ix = spl_token_2022_interface::instruction::burn_checked(
        &spl_token_2022_interface::ID,
        &account,
        &input.mint,
        &input.authority.pubkey(),
        &[],
        input.amount,
        input.decimals,
    )?;

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: [ix].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, account })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_instruction() {
        let mint = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let account = derive_ata(&authority, &mint);

        let ix = spl_token_2022_interface::instruction::burn_checked(
            &spl_token_2022_interface::ID,
            &account,
            &mint,
            &authority,
            &[],
            1000,
            9,
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token_2022_interface::ID);
        assert!(!ix.data.is_empty());
    }

    #[test]
    fn test_delegate_account_override() {
        // When account_owner is set, the burn target must derive from it, not from authority.
        let mint = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();
        let account_owner = Pubkey::new_unique();
        let account = derive_ata(&account_owner, &mint);

        assert_ne!(account, derive_ata(&delegate, &mint));

        let ix = spl_token_2022_interface::instruction::burn_checked(
            &spl_token_2022_interface::ID,
            &account,
            &mint,
            &delegate,
            &[],
            1000,
            9,
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token_2022_interface::ID);
    }
}
