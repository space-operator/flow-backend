use crate::prelude::*;
use flow_lib::command::prelude::*;
use mpl_hybrid::instructions::MigrateNftV1Builder;

const NAME: &str = "migrate_nft_v1";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_404/migrate_nft_v1.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    fee_payer: Wallet,
    authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    asset: Pubkey,
    #[serde_as(as = "AsPubkey")]
    collection: Pubkey,
    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Output {
    #[serde_as(as = "Option<AsSignature>")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    tracing::info!("input: {:?}", input);

    let (escrow_new, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"escrow", input.authority.pubkey().as_ref()],
        &mpl_hybrid::ID,
    );

    let (escrow_old, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"escrow", input.collection.as_ref()],
        &mpl_hybrid::ID,
    );

    let migrate_nft_v1_ix = MigrateNftV1Builder::new()
        .authority(input.authority.pubkey())
        .escrow_new(escrow_new)
        .escrow_old(escrow_old)
        .asset(input.asset)
        .collection(input.collection)
        .mpl_core(mpl_core::ID)
        .instruction();

    let ix = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: [migrate_nft_v1_ix].into(),
    };

    let signature = ctx.execute(ix, value::map! {}).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use crate::mpl_404::utils::{
        create_asset_v2, create_collection_v2, init_escrow_v1, init_escrow_v2, transfer_sol,
        CreateAssetV2Accounts, CreateAssetV2Args, CreateCollectionV2Accounts,
        CreateCollectionV2Args, InitEscrowV1Accounts, InitEscrowV1Args, InitEscrowV2Accounts,
    };

    use super::*;

    async fn setup(
        ctx: &Context,
        payer: Wallet,
        authority: Wallet,
        collection: Wallet,
        token: &Pubkey,
        fee_wallet: &Pubkey,
        escrow_name: String,
        uri: String,
        max: u64,
        min: u64,
        path: u16,
    ) -> (Pubkey, Pubkey, Pubkey) {
        // transfer sol to authority
        let transfer_sol_signature = transfer_sol(
            ctx,
            payer.clone(),
            authority.pubkey(),
            rust_decimal_macros::dec!(0.03),
        )
        .await
        .unwrap();

        dbg!(transfer_sol_signature);

        // create_collection
        let create_collection_signature = create_collection_v2(
            ctx,
            CreateCollectionV2Accounts {
                payer: payer.clone(),
                update_authority: Some(authority.pubkey()),
                collection: collection.clone(),
            },
            CreateCollectionV2Args {
                name: String::from("collection name"),
                uri: String::from("https://example.com"),
            },
        )
        .await
        .unwrap();

        dbg!(create_collection_signature);

        // init escrow
        let (escrow_v1, init_escrow_v1_signature) = init_escrow_v1(
            ctx,
            InitEscrowV1Accounts {
                payer: payer.clone(),
                authority: authority.clone(),
                collection: collection.clone(),
                token: *token,
                fee_location: *fee_wallet,
            },
            InitEscrowV1Args {
                fee_token_decimals: 9,
                name: escrow_name,
                uri,
                max,
                min,
                path,
                amount: rust_decimal_macros::dec!(0),
                fee_amount: rust_decimal_macros::dec!(0),
                sol_fee_amount: rust_decimal_macros::dec!(0),
            },
        )
        .await
        .unwrap();

        dbg!(init_escrow_v1_signature);

        // init_escrow_v2
        let (escrow_v2, init_escrow_v2_signature) = init_escrow_v2(
            ctx,
            InitEscrowV2Accounts {
                payer: payer.clone(),
                authority: authority.clone(),
            },
        )
        .await
        .unwrap();

        dbg!(init_escrow_v2_signature);

        // create asset
        let asset = Wallet::Keypair(Keypair::new());
        let create_asset_signature = create_asset_v2(
            ctx,
            CreateAssetV2Accounts {
                payer: payer.clone(),
                asset: asset.clone(),
                collection: Some(collection.pubkey()),
                authority: Some(authority.clone()),
                owner: Some(escrow_v1),
            },
            CreateAssetV2Args {
                name: String::from("asset name"),
                uri: String::from("https://example.com"),
            },
        )
        .await
        .unwrap();

        dbg!(create_asset_signature);

        (escrow_v1, escrow_v2, asset.pubkey())
    }

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_run() {
        let ctx = Context::default();

        // setup
        let fee_payer = Wallet::Keypair(Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ"));
        let authority = Wallet::Keypair(Keypair::new());
        let collection = Wallet::Keypair(Keypair::new());
        let token = solana_sdk::pubkey!("AdaQ1MKbeKDyXCSnuCtqs5MW9FaY1UMGtpCGbZnpbTbj");
        let fee_wallet = Keypair::new().pubkey();
        let name = String::from("Escrow Name");
        let uri = String::from("https://base.spaceoperator.com/storage/v1/object/public/blings_gg_nft/asset_metadata.json");
        let max = 1_u64;
        let min = 0_u64;
        let path = 1;

        let (_escrow_v1, _escrow_v2, asset) = setup(
            &ctx,
            fee_payer.clone(),
            authority.clone(),
            collection.clone(),
            &token,
            &fee_wallet,
            name,
            uri,
            max,
            min,
            path,
        )
        .await;

        let output = run(
            ctx,
            super::Input {
                fee_payer: fee_payer.clone(),
                authority: authority.clone(),
                asset,
                collection: collection.pubkey(),
                submit: true,
            },
        )
        .await
        .unwrap();

        dbg!(output.signature.unwrap());
    }
}
