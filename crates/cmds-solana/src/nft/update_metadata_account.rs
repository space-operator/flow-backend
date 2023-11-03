use crate::prelude::*;

const NAME: &str = "update_metadata_account";

inventory::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str =
        include_str!("../../../../node-definitions/solana/NFT/update_metadata_account.json");
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[derive(Deserialize, Debug)]
struct Input {
    #[serde(with = "value::keypair")]
    fee_payer: Keypair,
    #[serde(with = "value::pubkey")]
    mint_account: Pubkey,
    #[serde(with = "value::keypair")]
    update_authority: Keypair,
    #[serde(default, with = "value::pubkey::opt")]
    new_update_authority: Option<Pubkey>,
    data: Option<super::NftDataV2>,
    primary_sale_happen: Option<bool>,
    is_mutable: Option<bool>,
}

#[derive(Serialize, Debug)]
struct Output0 {
    #[serde(with = "value::pubkey")]
    metadata_account: Pubkey,
}

#[derive(Serialize, Debug)]
struct Output1 {
    #[serde(with = "value::signature")]
    signature: Signature,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output1, CommandError> {
    let (metadata_account, _) = mpl_token_metadata::pda::find_metadata_account(&input.mint_account);
    let signature = ctx
        .execute(
            Instructions {
                fee_payer: input.fee_payer.pubkey(),
                signers: [
                    input.fee_payer.clone_keypair(),
                    input.update_authority.clone_keypair(),
                ]
                .into(),
                minimum_balance_for_rent_exemption: 0,
                instructions: [
                    mpl_token_metadata::instruction::update_metadata_accounts_v2(
                        mpl_token_metadata::id(),
                        metadata_account,
                        input.update_authority.pubkey(),
                        input.new_update_authority,
                        input.data.map(Into::into),
                        input.primary_sale_happen,
                        input.is_mutable,
                    ),
                ]
                .into(),
            },
            value::to_map(&Output0 { metadata_account }).unwrap(),
        )
        .await?
        .signature
        .expect("instructions is not empty");

    Ok(Output1 { signature })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[test]
    fn test_minimal_input() {
        value::from_map::<Input>(value::map! {
            "fee_payer" => Keypair::new(),
            "mint_account" => Pubkey::new_unique(),
            "update_authority" => Keypair::new(),
        })
        .unwrap();
    }
}
