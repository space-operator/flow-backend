use crate::prelude::*;
use spl_token_2022_interface::instruction::AuthorityType;

const NAME: &str = "set_authority_t22";
const DEFINITION: &str = flow_lib::node_definition!("spl_token_2022/set_authority.jsonc");

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
    pub account_or_mint: Pubkey,
    pub current_authority: Wallet,
    pub authority_type: AuthorityType,
    #[serde_as(as = "Option<AsPubkey>")]
    pub new_authority: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let ix = spl_token_2022_interface::instruction::set_authority(
        &spl_token_2022_interface::ID,
        &input.account_or_mint,
        input.new_authority.as_ref(),
        input.authority_type,
        &input.current_authority.pubkey(),
        &[],
    )?;

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.current_authority].into(),
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
        let account_or_mint = Pubkey::new_unique();
        let current_authority = Pubkey::new_unique();
        let new_authority = Pubkey::new_unique();

        let ix = spl_token_2022_interface::instruction::set_authority(
            &spl_token_2022_interface::ID,
            &account_or_mint,
            Some(&new_authority),
            AuthorityType::MintTokens,
            &current_authority,
            &[],
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token_2022_interface::ID);
        assert!(!ix.data.is_empty());
    }
}
