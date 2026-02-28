use crate::prelude::*;
use super::derive_ata;

const NAME: &str = "mint_to_checked";
const DEFINITION: &str = flow_lib::node_definition!("spl_token_2022/mint_to_checked.jsonc");

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
    #[serde_as(as = "AsPubkey")]
    pub recipient_owner: Pubkey,
    pub mint_authority: Wallet,
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
    let account = derive_ata(&input.recipient_owner, &input.mint);

    let ix = spl_token_2022_interface::instruction::mint_to_checked(
        &spl_token_2022_interface::ID,
        &input.mint,
        &account,
        &input.mint_authority.pubkey(),
        &[],
        input.amount,
        input.decimals,
    )?;

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.mint_authority].into(),
        instructions: [ix].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
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
        let recipient_owner = Pubkey::new_unique();
        let mint_authority = Pubkey::new_unique();
        let account = derive_ata(&recipient_owner, &mint);

        let ix = spl_token_2022_interface::instruction::mint_to_checked(
            &spl_token_2022_interface::ID,
            &mint,
            &account,
            &mint_authority,
            &[],
            1000,
            9,
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token_2022_interface::ID);
        assert!(!ix.data.is_empty());
    }
}
