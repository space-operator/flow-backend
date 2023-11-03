use crate::prelude::*;

const NAME: &str = "approve_collection_authority";

inventory::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        include_str!("../../../../node-definitions/solana/NFT/approve_collection_authority.json");
    static CACHE: BuilderCache = BuilderCache::new(|| {
        Ok(CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")?)
    });
    Ok(CACHE.clone()?.build(run))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    pub new_collection_authority: Pubkey,
    #[serde(with = "value::keypair")]
    pub update_authority: Keypair,
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let program_id = mpl_token_metadata::id();

    let metadata_seeds = &[
        mpl_token_metadata::state::PREFIX.as_bytes(),
        program_id.as_ref(),
        input.mint_account.as_ref(),
    ];

    let (metadata_pubkey, _) = Pubkey::find_program_address(metadata_seeds, &program_id);

    let (collection_authority_record, _) =
        mpl_token_metadata::pda::find_collection_authority_account(
            &input.mint_account,
            &input.new_collection_authority,
        );

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_token_metadata::state::CollectionAuthorityRecord,
        >())
        .await?;

    let instruction = mpl_token_metadata::instruction::approve_collection_authority(
        mpl_token_metadata::id(),
        collection_authority_record,
        input.new_collection_authority,
        input.update_authority.pubkey(),
        input.fee_payer.pubkey(),
        metadata_pubkey,
        input.mint_account,
    );

    let instructions = if input.submit {
        Instructions {
            fee_payer: input.fee_payer.pubkey(),
            signers: [input.fee_payer, input.update_authority].into(),
            minimum_balance_for_rent_exemption,
            instructions: [instruction].into(),
        }
    } else {
        <_>::default()
    };

    let signature = ctx.execute(instructions, <_>::default()).await?.signature;

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
