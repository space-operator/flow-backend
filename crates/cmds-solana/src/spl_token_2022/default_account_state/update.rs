use super::DefaultAccountState;
use crate::prelude::*;
use spl_token_2022_interface::state::AccountState;

const NAME: &str = "update_default_account_state";
const DEFINITION: &str = flow_lib::node_definition!("spl_token_2022/default_account_state/update.jsonc");

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
    pub freeze_authority: Wallet,
    pub state: DefaultAccountState,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let state: AccountState = input.state.into();
    let ix = spl_token_2022_interface::extension::default_account_state::instruction::update_default_account_state(
        &spl_token_2022_interface::ID,
        &input.mint,
        &input.freeze_authority.pubkey(),
        &[],
        &state,
    )?;

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.freeze_authority].into(),
        instructions: [ix].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
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
    fn test_instruction() {
        let mint = Pubkey::new_unique();
        let freeze_authority = Pubkey::new_unique();
        let state = spl_token_2022_interface::state::AccountState::Frozen;

        let ix = spl_token_2022_interface::extension::default_account_state::instruction::update_default_account_state(
            &spl_token_2022_interface::ID,
            &mint,
            &freeze_authority,
            &[],
            &state,
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token_2022_interface::ID);
        assert!(!ix.data.is_empty());
    }
}
