use crate::prelude::*;

const NAME: &str = "initialize_transfer_fee_config";
const DEFINITION: &str = flow_lib::node_definition!("spl_token_2022/transfer_fee/initialize.jsonc");

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
    #[serde_as(as = "Option<AsPubkey>")]
    pub transfer_fee_config_authority: Option<Pubkey>,
    #[serde_as(as = "Option<AsPubkey>")]
    pub withdraw_withheld_authority: Option<Pubkey>,
    pub transfer_fee_basis_points: u16,
    pub maximum_fee: u64,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let ix =
        spl_token_2022_interface::extension::transfer_fee::instruction::initialize_transfer_fee_config(
            &spl_token_2022_interface::ID,
            &input.mint,
            input.transfer_fee_config_authority.as_ref(),
            input.withdraw_withheld_authority.as_ref(),
            input.transfer_fee_basis_points,
            input.maximum_fee,
        )?;

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
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
        let config_authority = Pubkey::new_unique();
        let withdraw_authority = Pubkey::new_unique();

        let ix = spl_token_2022_interface::extension::transfer_fee::instruction::initialize_transfer_fee_config(
            &spl_token_2022_interface::ID,
            &mint,
            Some(&config_authority),
            Some(&withdraw_authority),
            100,
            1_000_000,
        )
        .unwrap();

        assert_eq!(ix.program_id, spl_token_2022_interface::ID);
        assert!(!ix.data.is_empty());
    }
}
