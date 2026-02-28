use crate::prelude::*;
use crate::spl_token_2022::derive_ata;

const NAME: &str = "withdraw_withheld_tokens_from_mint";
const DEFINITION: &str =
    flow_lib::node_definition!("spl_token_2022/transfer_fee/withdraw_from_mint.jsonc");

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
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub destination: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let destination = derive_ata(&input.authority.pubkey(), &input.mint);

    let ix = spl_token_2022_interface::extension::transfer_fee::instruction::withdraw_withheld_tokens_from_mint(
        &spl_token_2022_interface::ID,
        &input.mint,
        &destination,
        &input.authority.pubkey(),
        &[],
    )?;

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: [ix].into(),
    };

    let ins = if input.submit { ins } else { Default::default() };
    let signature = ctx.execute(ins, <_>::default()).await?.signature;
    Ok(Output { signature, destination })
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
        let destination = derive_ata(&authority, &mint);

        let ix = spl_token_2022_interface::extension::transfer_fee::instruction::withdraw_withheld_tokens_from_mint(
            &spl_token_2022_interface::ID,
            &mint,
            &destination,
            &authority,
            &[],
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token_2022_interface::ID);
        assert!(!ix.data.is_empty());
    }
}
