use crate::prelude::*;
use mpl_token_metadata::{
    accounts::{MasterEdition, Metadata},
    instructions::CreateV1InstructionArgs,
    types::Collection,
};
use solana_program::{system_program, sysvar};

use crate::nft::{
    CollectionDetails, NftCollection, NftCreator, NftDataV2, NftUses, PrintSupply, TokenStandard,
};

// Command Name
const NAME: &str = "create_v1";

const DEFINITION: &str = flow_lib::node_definition!("nft/v1/create_v1.json");

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
    update_authority: Wallet,
    pub mint_account: Wallet,
    #[serde(with = "value::pubkey")]
    pub mint_authority: Pubkey,
    pub data: NftDataV2,
    pub print_supply: Option<u64>,
    #[serde(default, with = "value::pubkey::opt")]
    pub collection_mint_account: Option<Pubkey>,
    pub collection_details: Option<CollectionDetails>,
    pub is_mutable: bool,
    pub token_standard: String,
    pub decimals: Option<u8>,
    pub creators: Option<Vec<NftCreator>>,
    pub uses: Option<NftUses>,
    #[serde(default, with = "value::pubkey::opt")]
    pub rule_set: Option<Pubkey>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let (metadata_account, _) = Metadata::find_pda(&input.mint_account.pubkey());

    let (master_edition_account, _) = MasterEdition::find_pda(&input.mint_account.pubkey());

    // // get associated token account pda
    // let token_account = spl_associated_token_account::get_associated_token_address(
    //     &input.fee_payer.pubkey(),
    //     &input.mint_account.pubkey(),
    // );

    let create_ix = mpl_token_metadata::instructions::CreateV1 {
        metadata: metadata_account,
        master_edition: Some(master_edition_account),
        mint: (input.mint_account.pubkey(), true),
        authority: input.mint_authority,
        payer: input.fee_payer.pubkey(),
        update_authority: (input.update_authority.pubkey(), true),
        system_program: system_program::id(),
        sysvar_instructions: sysvar::instructions::id(),
        spl_token_program: Some(spl_token::id()),
    };

    // Creators
    let creators_input = input.creators.map(|creators| {
        creators
            .into_iter()
            .map(|creator| creator.into())
            .collect::<Vec<mpl_token_metadata::types::Creator>>()
    });

    let creators_data = input.data.creators.map(|creators| {
        creators
            .into_iter()
            .map(|creator| creator.into())
            .collect::<Vec<mpl_token_metadata::types::Creator>>()
    });

    let creators = creators_input.or(creators_data);

    // Uses
    let uses = input
        .uses
        .map(Into::into)
        .or_else(|| input.data.uses.map(Into::into));

    // Token Standard
    let token_standard: TokenStandard = input.token_standard.into();
    let token_standard: mpl_token_metadata::types::TokenStandard = token_standard.into();

    // Collection
    let collection = input
        .collection_mint_account
        .map(|key| {
            Collection::from(NftCollection {
                verified: false,
                key,
            })
        })
        .or(input.data.collection.map(Into::into));

    // Print Supply
    let print_supply = match input.print_supply {
        Some(_) => {
            let print_supply: PrintSupply = input.print_supply.into();
            let print_supply: mpl_token_metadata::types::PrintSupply = print_supply.into();
            Some(print_supply)
        }
        None => None,
    };

    let args = CreateV1InstructionArgs {
        name: input.data.name,
        symbol: input.data.symbol,
        uri: input.data.uri,
        seller_fee_basis_points: input.data.seller_fee_basis_points,
        creators,
        primary_sale_happened: false,
        is_mutable: input.is_mutable,
        token_standard,
        collection,
        uses,
        collection_details: input.collection_details.map(|details| details.into()),
        rule_set: input.rule_set,
        decimals: input.decimals,
        print_supply,
    };

    let create_ix = create_ix.instruction(args);

    let mint_account_pubkey = input.mint_account.pubkey();

    let ins = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.update_authority, input.mint_account].into(),
        instructions: [create_ix].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "metadata_account" => metadata_account,
                "master_edition_account" => master_edition_account,
                "mint_account" => mint_account_pubkey,
                // "token"=> token_account,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}
