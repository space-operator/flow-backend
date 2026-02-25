use crate::prelude::*;
use mpl_token_metadata::{
    accounts::{Metadata, TokenRecord},
    types::TokenStandard,
};

// Command Name
const NAME: &str = "burn_v1";

const DEFINITION: &str = flow_lib::node_definition!("mpl_token_metadata/burn_v1.jsonc");

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
    pub fee_payer: Wallet,
    authority: Wallet,
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    pub amount: Option<u64>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (metadata_account, _) = Metadata::find_pda(&input.mint_account);

    // // get associated token account pda
    let token_account =
        spl_associated_token_account_interface::address::get_associated_token_address(
            &input.authority.pubkey(),
            &input.mint_account,
        );

    let mut create_ix_builder = mpl_token_metadata::instructions::BurnV1Builder::new();
    create_ix_builder
        .authority(input.authority.pubkey())
        .mint(input.mint_account)
        .metadata(metadata_account)
        .token(token_account);

    if let Some(acc) = ctx
        .solana_client()
        .get_account_with_commitment(&metadata_account, ctx.solana_client().commitment())
        .await?
        .value
    {
        let metadata = Metadata::safe_deserialize(&acc.data)?;

        if let Some(standard) = metadata.token_standard
            && standard == TokenStandard::ProgrammableNonFungible
        {
            let token_record = Some(TokenRecord::find_pda(&input.mint_account, &token_account).0);
            create_ix_builder.token_record(token_record);
        };
    }

    if let Some(amount) = input.amount {
        create_ix_builder.amount(amount);
    };

    // if let Some(collection_metadata) = input.collection_metadata {
    //     create_ix_builder.collection_metadata(Some(collection_metadata));
    // };

    // TODO implement editions burning
    // https://github.com/metaplex-foundation/mpl-token-metadata/blob/main/programs/token-metadata/program/tests/utils/digital_asset.rs
    // https://github.com/metaplex-foundation/mpl-token-metadata/blob/main/programs/token-metadata/program/tests/burn.rs
    // let (master_edition_account, _) = MasterEdition::find_pda(&input.mint_account);
    // if let Some(edition) = input.edition {
    //     create_ix_builder.edition(input.edition);
    // };

    let create_ix = create_ix_builder.instruction();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: [create_ix].into(),
    };

    let ins = if input.submit {
        ins
    } else {
        Default::default()
    };

    let signature = ctx.execute(ins, <_>::default()).await?.signature;

    Ok(Output { signature })
}
