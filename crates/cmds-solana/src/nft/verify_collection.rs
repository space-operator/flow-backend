use crate::prelude::*;

// Command Name
const NAME: &str = "verify_collection";

const DEFINITION: &str = flow_lib::node_definition!("NFT/verify_collection.json");

fn build() -> BuildResult {
    use once_cell::sync::Lazy;
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
        mpl_token_metadata::pda::find_metadata_account(&input.collection_mint_account);

    let (collection_master_edition_account, _) =
        mpl_token_metadata::pda::find_master_edition_account(&input.collection_mint_account);

    let collection_authority_record = if input.collection_authority_is_delegated {
        Some(
            mpl_token_metadata::pda::find_collection_authority_account(
                &input.mint_account,
                &input.collection_authority.pubkey(),
            )
            .0,
        )
    } else {
        None
    };

    let (metadata_account, _) = mpl_token_metadata::pda::find_metadata_account(&input.mint_account);

    let minimum_balance_for_rent_exemption = ctx
        .solana_client()
        .get_minimum_balance_for_rent_exemption(
            100, // std::mem::size_of::<
                // mpl_token_metadata::state::VerifyCollection,
                // >(),
        )
        .await?;

    let instructions = vec![
        mpl_token_metadata::instruction::verify_sized_collection_item(
            mpl_token_metadata::id(),
            metadata_account,
            input.collection_authority.pubkey(),
            input.fee_payer.pubkey(),
            input.collection_mint_account,
            collection_metadata_account,
            collection_master_edition_account,
            collection_authority_record,
        ),
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
        minimum_balance_for_rent_exemption,
    };

    let signature: Option<Signature> = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
