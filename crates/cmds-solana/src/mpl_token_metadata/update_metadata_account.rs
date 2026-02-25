use crate::prelude::*;
use ::mpl_token_metadata::accounts::Metadata;
use ::mpl_token_metadata::instructions::UpdateMetadataAccountV2Builder;

const NAME: &str = "update_metadata_account";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_token_metadata/update_metadata_account.jsonc");
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

#[derive(Deserialize, Debug)]
struct Input {
    fee_payer: Wallet,
    #[serde(with = "value::pubkey")]
    mint_account: Pubkey,
    update_authority: Wallet,
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

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output1, CommandError> {
    let (metadata_account, _) = Metadata::find_pda(&input.mint_account);

    let mut builder = UpdateMetadataAccountV2Builder::new();
    builder
        .metadata(metadata_account)
        .update_authority(input.update_authority.pubkey());
    if let Some(data) = input.data {
        builder.data(data.into());
    }
    if let Some(key) = input.new_update_authority {
        builder.new_update_authority(key);
    }
    if let Some(v) = input.primary_sale_happen {
        builder.primary_sale_happened(v);
    }
    if let Some(v) = input.is_mutable {
        builder.is_mutable(v);
    }
    let instruction = builder.instruction();

    let signature = ctx
        .execute(
            Instructions {
lookup_tables: None,
                fee_payer: input.fee_payer.pubkey(),
                signers: [
                    input.fee_payer,
                    input.update_authority,
                ]
                .into(),
                instructions: [instruction].into(),
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
