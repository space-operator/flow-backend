use crate::prelude::*;
use crate::spl_token_2022::derive_ata;

const NAME: &str = "transfer_checked_with_fee";
const DEFINITION: &str =
    flow_lib::node_definition!("spl_token_2022/transfer_fee/transfer_checked_with_fee.jsonc");

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
    pub destination_owner: Pubkey,
    pub authority: Wallet,
    pub amount: u64,
    pub decimals: u8,
    pub fee: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
    #[serde_as(as = "AsPubkey")]
    pub source: Pubkey,
    #[serde_as(as = "AsPubkey")]
    pub destination: Pubkey,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let source = derive_ata(&input.authority.pubkey(), &input.mint);
    let destination = derive_ata(&input.destination_owner, &input.mint);

    let ix =
        spl_token_2022_interface::extension::transfer_fee::instruction::transfer_checked_with_fee(
            &spl_token_2022_interface::ID,
            &source,
            &input.mint,
            &destination,
            &input.authority.pubkey(),
            &[],
            input.amount,
            input.decimals,
            input.fee,
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
    Ok(Output {
        signature,
        source,
        destination,
    })
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
        let destination_owner = Pubkey::new_unique();
        let source = derive_ata(&authority, &mint);
        let destination = derive_ata(&destination_owner, &mint);

        let ix = spl_token_2022_interface::extension::transfer_fee::instruction::transfer_checked_with_fee(
            &spl_token_2022_interface::ID,
            &source,
            &mint,
            &destination,
            &authority,
            &[],
            1000,
            9,
            50,
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token_2022_interface::ID);
        assert!(!ix.data.is_empty());
    }
}
