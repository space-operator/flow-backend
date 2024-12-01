use crate::{mpl_404::constants::{FEE_WALLET, SLOT_HASHES}, prelude::*};
use flow_lib::command::prelude::*;
use mpl_hybrid::instructions::CaptureV2Builder;

const NAME: &str = "capture_v2";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_404/capture_v2.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    // account
    owner: Wallet,
    authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    asset: Pubkey,
    #[serde_as(as = "AsPubkey")]
    collection: Pubkey,
    #[serde_as(as = "AsPubkey")]
    token: Pubkey,
    #[serde_as(as = "AsPubkey")]
    fee_project_account: Pubkey,

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

    let (recipe, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"recipe", input.collection.as_ref()],
        &mpl_hybrid::ID,
    );

    let (escrow, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"escrow", input.authority.pubkey().as_ref()],
        &mpl_hybrid::ID,
    );

    // must already be initialized
    let user_token_account = spl_associated_token_account::get_associated_token_address(
        &input.owner.pubkey(),
        &input.token,
    );

    let escrow_token_account =
        spl_associated_token_account::get_associated_token_address(&escrow, &input.token);

    let fee_token_account = spl_associated_token_account::get_associated_token_address(
        &input.fee_project_account,
        &input.token,
    );

    let capture_v2_ix = CaptureV2Builder::new()
        .owner(input.owner.pubkey())
        .authority(input.authority.pubkey(), true)
        .recipe(recipe)
        .escrow(escrow)
        .asset(input.asset)
        .collection(input.collection)
        .token(input.token)
        .user_token_account(user_token_account)
        .escrow_token_account(escrow_token_account)
        .fee_token_account(fee_token_account)
        .fee_project_account(input.fee_project_account)
        .fee_sol_account(FEE_WALLET)
        .recent_blockhashes(SLOT_HASHES)
        .mpl_core(mpl_core::ID)
        .associated_token_program(spl_associated_token_account::ID)
        .instruction();

    let ix = Instructions {
        lookup_tables: None,
        fee_payer: input.owner.pubkey(),
        signers: [input.owner, input.authority].into(),
        instructions: [capture_v2_ix].into(),
    };

    let signature = ctx.execute(ix, value::map! {}).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use crate::mpl_404::utils::{
        create_asset_v2, create_collection_v2, init_ata_if_needed, init_escrow_v2, init_recipe_v1,
        transfer_sol, CreateAssetV2Accounts, CreateAssetV2Args, CreateCollectionV2Accounts,
        CreateCollectionV2Args, InitEscrowV2Accounts, InitRecipeAccounts, InitRecipeArgs,
    };

    use super::*;

    async fn setup(
        ctx: &Context,
        payer: Wallet,
        authority: Wallet,
        collection: Wallet,
        asset: Wallet,
        token: Pubkey,
        fee_location: Pubkey,
        fee_token_decimals: u8,
    ) {
        // transfer some sol to authority for creating token account
        let transfer_sol_signature = transfer_sol(
            ctx,
            payer.clone(),
            authority.pubkey(),
            rust_decimal_macros::dec!(0.03),
        )
        .await
        .unwrap();

        dbg!(transfer_sol_signature);

        // create collection v2
        let create_collection_v2_signature = create_collection_v2(
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

        dbg!(create_collection_v2_signature);

        // init escrow v2
        let (escrow, init_escrow_v2_signature) = init_escrow_v2(
            ctx,
            InitEscrowV2Accounts {
                payer: payer.clone(),
                authority: authority.clone(),
            },
        )
        .await
        .unwrap();

        dbg!(init_escrow_v2_signature);

        // create asset v2
        let create_asset_v2_signature = create_asset_v2(
            ctx,
            CreateAssetV2Accounts {
                payer: payer.clone(),
                asset: asset.clone(),
                collection: Some(collection.pubkey()),
                authority: Some(authority.clone()),
                owner: Some(escrow),
            },
            CreateAssetV2Args {
                name: String::from("asset name"),
                uri: String::from("https://example.com"),
            },
        )
        .await
        .unwrap();

        dbg!(create_asset_v2_signature);

        // init recipe
        let init_recipe_signature = init_recipe_v1(
            ctx,
            InitRecipeAccounts {
                payer: payer.clone(),
                authority: authority.clone(),
                collection: collection.pubkey(),
                token,
                fee_location,
            },
            InitRecipeArgs {
                fee_token_decimals,
                name: String::from("recipe name"),
                uri: String::from("https://example.com"),
                max: 1_u64,
                min: 0_u64,
                amount: rust_decimal_macros::dec!(0),
                fee_amount_capture: rust_decimal_macros::dec!(0),
                fee_amount_release: rust_decimal_macros::dec!(0),
                sol_fee_amount_capture: rust_decimal_macros::dec!(0),
                sol_fee_amount_release: rust_decimal_macros::dec!(0),
                path: 1_u16,
            },
        )
        .await
        .unwrap();

        dbg!(init_recipe_signature);

        // init ata if needed
        let init_ata_if_needed_signature =
            init_ata_if_needed(ctx, payer.clone(), payer.pubkey(), token)
                .await
                .unwrap();

        dbg!(init_ata_if_needed_signature);
    }

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_run() {
        let ctx = Context::default();

        let fee_payer = Wallet::Keypair(Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ"));
        let authority = Wallet::Keypair(Keypair::new());
        let fee_token_decimals = 9_u8;
        let collection = Wallet::Keypair(Keypair::new());
        let asset = Wallet::Keypair(Keypair::new());
        let token = solana_sdk::pubkey!("AdaQ1MKbeKDyXCSnuCtqs5MW9FaY1UMGtpCGbZnpbTbj");
        let fee_location = Wallet::Keypair(Keypair::new());

        setup(
            &ctx,
            fee_payer.clone(),
            authority.clone(),
            collection.clone(),
            asset.clone(),
            token,
            fee_location.pubkey(),
            fee_token_decimals,
        )
        .await;

        let output = run(
            ctx,
            super::Input {
                owner: fee_payer.clone(),
                authority: authority.clone(),
                asset: asset.pubkey(),
                collection: collection.pubkey(),
                token,
                fee_project_account: fee_location.pubkey(),
                submit: true,
            },
        )
        .await
        .unwrap();

        dbg!(output.signature.unwrap());
    }
}
