use super::{CollectionDetails, NftCreator, NftDataV2, NftUses};
use crate::prelude::*;
use mpl_token_metadata::accounts::Metadata;
use solana_program::system_program;
use solana_sdk::pubkey::Pubkey;

// Command Name
const NAME: &str = "create_metadata_account";

const DEFINITION: &str =
    include_str!("../../../../node-definitions/solana/NFT/create_metadata_account.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .simple_instruction_info("signature")
    });
    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub update_authority: Keypair,
    pub is_mutable: bool,
    #[serde(with = "value::pubkey")]
    pub mint_account: Pubkey,
    #[serde(with = "value::pubkey")]
    pub mint_authority: Pubkey,
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
    pub metadata: NftDataV2,
    pub metadata_uri: String,
    pub uses: Option<NftUses>,
    #[serde(default, with = "value::pubkey::opt")]
    pub collection_mint_account: Option<Pubkey>,
    pub creators: Vec<NftCreator>,
    pub collection_details: Option<CollectionDetails>,
    #[serde(default = "value::default::bool_true")]
    pub submit: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(default, with = "value::signature::opt")]
    signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let (metadata_account, _) = Metadata::find_pda(&input.mint_account);

    let minimum_balance_for_rent_exemption = ctx
        .solana_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            mpl_token_metadata::accounts::Metadata,
        >())
        .await?;

    let create_ix = mpl_token_metadata::instructions::CreateMetadataAccountV3 {
        metadata: metadata_account,
        mint: input.mint_account,
        mint_authority: input.mint_authority,
        payer: input.fee_payer.pubkey(),
        update_authority: (input.update_authority.pubkey(), true),
        system_program: system_program::id(),
        //TODO double check what this is for
        rent: Some(input.fee_payer.pubkey()),
    };

    let args = mpl_token_metadata::instructions::CreateMetadataAccountV3InstructionArgs {
        data: input.metadata.into(),
        is_mutable: input.is_mutable,
        collection_details: input.collection_details.map(|details| details.into()),
    };

    let ins = create_ix.instruction(args);

    let ins = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.update_authority.clone_keypair(),
        ]
        .into(),
        instructions: [ins].into(),
        minimum_balance_for_rent_exemption,
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx
        .execute(
            ins,
            value::map! {
                "metadata_account" => metadata_account,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_inputs() {
//         let metadata: Value = serde_json::from_str::<serde_json::Value>(
//             r#"
// {
//     "name": "SO #11111",
//     "symbol": "SPOP",
//     "description": "Space Operator is a dynamic PFP collection",
//     "seller_fee_basis_points": 250,
//     "image": "https://arweave.net/vb1tD7tfAyrhZceA1MOYvvyqzZWgzHGDVZF37yDNH1Q",
//     "attributes": [
//         {
//             "trait_type": "Season",
//             "value": "Fall"
//         },
//         {
//             "trait_type": "Light Color",
//             "value": "Orange"
//         }
//     ],
//     "properties": {
//         "files": [
//             {
//                 "uri": "https://arweave.net/vb1tD7tfAyrhZceA1MOYvvyqzZWgzHGDVZF37yDNH1Q",
//                 "type": "image/jpeg"
//             }
//         ],
//         "category": null
//     }
// }"#,
//         )
//         .unwrap()
//         .into();
//         let uses: Value = serde_json::from_str::<serde_json::Value>(
//             r#"
// {
// "use_method": "Burn",
// "remaining": 500,
// "total": 500
// }
// "#,
//         )
//         .unwrap()
//         .into();
//         let creators: Value = serde_json::from_str::<serde_json::Value>(
//             r#"
// [{
// "address": "DpfvhHU7z1CK8eP5xbEz8c4WBNHUfqUVtAE7opP2kJBc",
// "share": 100
// }]"#,
//         )
//         .unwrap()
//         .into();
//         let inputs = value::map! {
//             PROXY_AS_UPDATE_AUTHORITY => "3G3ixjPdvg7NhazP932tCk88jgLJLzaDBe84mPa43Zyp",
//             IS_MUTABLE => true,
//             MINT_ACCOUNT => "C3EbZLYQ7Axv4PS9o4s4bSruFaiAVcynHZYds18VyWdZ",
//             MINT_AUTHORITY => "C3EbZLYQ7Axv4PS9o4s4bSruFaiAVcynHZYds18VyWdZ",
//             FEE_PAYER => "5s8bKTTgKLh2TudJBQwU6sx9DfFEtHcBP85aYZquEsqHrvipcWWCXxuyz4fsGsxTZ8NGMqMHFowUoQcoqcJSwLrP",
//             METADATA => metadata,
//             METADATA_URI => "https://arweave.net/3FxpIIbpySnfTTXIrpojhF2KHHjevI8Mrt3pACmEbSY",
//             USES => uses,
//             CREATORS => creators,
//         };
//         let inputs: Input = value::from_map(inputs).unwrap();
//         dbg!(inputs);
//     }
// }
