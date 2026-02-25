use crate::prelude::*;
use ::mpl_token_metadata::accounts::{CollectionAuthorityRecord, Metadata};
use ::mpl_token_metadata::instructions::ApproveCollectionAuthorityBuilder;

const NAME: &str = "approve_collection_authority";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_token_metadata/approve_collection_authority.jsonc");
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
    pub update_authority: Wallet,
    pub fee_payer: Wallet,
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

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (metadata_pubkey, _) = Metadata::find_pda(&input.mint_account);

    let (collection_authority_record, _) = CollectionAuthorityRecord::find_pda(
        &input.mint_account,
        &input.new_collection_authority,
    );

    let instruction = ApproveCollectionAuthorityBuilder::new()
        .collection_authority_record(collection_authority_record)
        .new_collection_authority(input.new_collection_authority)
        .update_authority(input.update_authority.pubkey())
        .payer(input.fee_payer.pubkey())
        .metadata(metadata_pubkey)
        .mint(input.mint_account)
        .instruction();

    let instructions = if input.submit {
        Instructions {
lookup_tables: None,
            fee_payer: input.fee_payer.pubkey(),
            signers: [input.fee_payer, input.update_authority].into(),
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
