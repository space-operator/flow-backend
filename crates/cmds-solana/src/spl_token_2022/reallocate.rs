use crate::prelude::*;
use super::derive_ata;
use spl_token_2022_interface::extension::ExtensionType;

const NAME: &str = "reallocate_t22";
const DEFINITION: &str = flow_lib::node_definition!("spl_token_2022/reallocate.jsonc");

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
    pub payer: Wallet,
    pub owner: Wallet,
    pub extension_types: Vec<ExtensionType>,
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
    let account = derive_ata(&input.owner.pubkey(), &input.mint);

    let ix = spl_token_2022_interface::instruction::reallocate(
        &spl_token_2022_interface::ID,
        &account,
        &input.payer.pubkey(),
        &input.owner.pubkey(),
        &[],
        &input.extension_types,
    )?;

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.payer, input.owner].into(),
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
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let account = derive_ata(&owner, &mint);

        let ix = spl_token_2022_interface::instruction::reallocate(
            &spl_token_2022_interface::ID,
            &account,
            &payer,
            &owner,
            &[],
            &[ExtensionType::TransferFeeAmount, ExtensionType::ImmutableOwner],
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token_2022_interface::ID);
        assert!(!ix.data.is_empty());
    }
}
