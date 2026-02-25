use crate::prelude::*;
use ::mpl_token_metadata::accounts::{CollectionAuthorityRecord, MasterEdition, Metadata};
use ::mpl_token_metadata::instructions::VerifySizedCollectionItemBuilder;

// Command Name
const NAME: &str = "verify_collection";

const DEFINITION: &str = flow_lib::node_definition!("mpl_token_metadata/verify_collection.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    pub fee_payer: Wallet,
    pub collection_authority: Wallet,
    #[serde(with = "value::pubkey")]
    pub collection_mint_account: Pubkey,
    pub collection_authority_is_delegated: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (collection_metadata_account, _) =
        Metadata::find_pda(&input.collection_mint_account);

    let (collection_master_edition_account, _) =
        MasterEdition::find_pda(&input.collection_mint_account);

    let collection_authority_record = if input.collection_authority_is_delegated {
        Some(
            CollectionAuthorityRecord::find_pda(
                &input.mint_account,
                &input.collection_authority.pubkey(),
            )
            .0,
        )
    } else {
        None
    };

    let (metadata_account, _) = Metadata::find_pda(&input.mint_account);

    let instructions = vec![
        VerifySizedCollectionItemBuilder::new()
            .metadata(metadata_account)
            .collection_authority(input.collection_authority.pubkey())
            .payer(input.fee_payer.pubkey())
            .collection_mint(input.collection_mint_account)
            .collection(collection_metadata_account)
            .collection_master_edition_account(collection_master_edition_account)
            .collection_authority_record(collection_authority_record)
            .instruction(),
    ];

    let ins = Instructions {
lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer,
            input.collection_authority,
        ]
        .into(),
        instructions,
    };

    let signature: Option<Signature> = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
