use crate::{prelude::*, utils::ui_amount_to_amount};
use flow_lib::command::prelude::*;
use mpl_hybrid::instructions::UpdateRecipeV1Builder;

const NAME: &str = "update_recipe";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_404/update_recipe.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    fee_payer: Wallet,
    fee_token_decimals: u8,

    // accounts
    authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    collection: Pubkey,
    #[serde_as(as = "AsPubkey")]
    token: Pubkey,
    #[serde_as(as = "AsPubkey")]
    fee_location: Pubkey,

    // args
    name: Option<String>,
    uri: Option<String>,
    max: Option<u64>,
    min: Option<u64>,
    #[serde_as(as = "Option<AsDecimal>")]
    amount: Option<Decimal>,
    #[serde_as(as = "Option<AsDecimal>")]
    fee_amount_capture: Option<Decimal>,
    #[serde_as(as = "Option<AsDecimal>")]
    fee_amount_release: Option<Decimal>,
    #[serde_as(as = "Option<AsDecimal>")]
    sol_fee_amount_capture: Option<Decimal>,
    #[serde_as(as = "Option<AsDecimal>")]
    sol_fee_amount_release: Option<Decimal>,
    // CURRENT PATH OPTIONS:
    // 0-- NFT METADATA IS UPDATED ON SWAP
    // 1-- NFT METADATA IS NOT UPDATED ON SWAP
    path: Option<u16>,

    #[serde(default = "value::default::bool_true")]
    submit: bool,
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Output {
    #[serde_as(as = "AsPubkey")]
    pub recipe: Pubkey,
    #[serde_as(as = "Option<AsSignature>")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    tracing::info!("input: {:?}", input);

    let (recipe, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"recipe", input.collection.as_ref()],
        &mpl_hybrid::ID,
    );

    let mut builder = UpdateRecipeV1Builder::new();
    let mut update_recipe_ix = builder
        .recipe(recipe)
        .authority(input.authority.pubkey())
        .collection(input.collection)
        .token(input.token)
        .fee_location(input.fee_location);

    if let Some(name) = input.name {
        update_recipe_ix = update_recipe_ix.name(name);
    }

    if let Some(uri) = input.uri {
        update_recipe_ix = update_recipe_ix.uri(uri);
    }

    if let Some(max) = input.max {
        update_recipe_ix = update_recipe_ix.max(max);
    }

    if let Some(min) = input.min {
        update_recipe_ix = update_recipe_ix.min(min);
    }

    if let Some(amount) = input.amount {
        update_recipe_ix =
            update_recipe_ix.amount(ui_amount_to_amount(amount, input.fee_token_decimals)?);
    }

    if let Some(fee_amount_capture) = input.fee_amount_capture {
        update_recipe_ix = update_recipe_ix.fee_amount_capture(ui_amount_to_amount(
            fee_amount_capture,
            input.fee_token_decimals,
        )?);
    }

    if let Some(fee_amount_release) = input.fee_amount_release {
        update_recipe_ix = update_recipe_ix.fee_amount_release(ui_amount_to_amount(
            fee_amount_release,
            input.fee_token_decimals,
        )?);
    }

    if let Some(sol_fee_amount_capture) = input.sol_fee_amount_capture {
        update_recipe_ix = update_recipe_ix.sol_fee_amount_capture(ui_amount_to_amount(
            sol_fee_amount_capture,
            input.fee_token_decimals,
        )?);
    }

    if let Some(sol_fee_amount_release) = input.sol_fee_amount_release {
        update_recipe_ix = update_recipe_ix.sol_fee_amount_release(ui_amount_to_amount(
            sol_fee_amount_release,
            input.fee_token_decimals,
        )?);
    }

    if let Some(path) = input.path {
        update_recipe_ix = update_recipe_ix.path(path);
    }

    let ix = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer].into(),
        instructions: [update_recipe_ix.instruction()].into(),
    };

    let signature = ctx.execute(ix, value::map! {}).await?.signature;

    Ok(Output { recipe, signature })
}

#[cfg(test)]
mod tests {
    use mpl_hybrid::instructions::InitRecipeBuilder;

    use super::*;

    async fn create_collection(
        ctx: &Context,
        collection: &Wallet,
        payer: &Wallet,
        name: String,
        uri: String,
    ) -> crate::Result<Signature> {
        let ix = mpl_core::instructions::CreateCollectionV2Builder::new()
            .collection(collection.pubkey())
            .payer(payer.pubkey())
            .update_authority(Some(payer.pubkey()))
            .name(name)
            .uri(uri)
            .instruction();

        let (mut create_collection_tx, recent_blockhash) =
            execute(&ctx.solana_client, &payer.pubkey(), &[ix])
                .await
                .unwrap();

        create_collection_tx
            .try_sign(
                &[payer.keypair().unwrap(), collection.keypair().unwrap()],
                recent_blockhash,
            )
            .unwrap();

        submit_transaction(&ctx.solana_client, create_collection_tx).await
    }

    async fn init_recipe(
        ctx: &Context,
        fee_token_decimals: u8,
        collection: &Wallet,
        token: &Pubkey,
        fee_location: &Pubkey,
        payer: &Wallet,
        name: String,
        uri: String,
        max: u64,
        min: u64,
        amount: Decimal,
        fee_amount_capture: Decimal,
        fee_amount_release: Decimal,
        sol_fee_amount_capture: Decimal,
        sol_fee_amount_release: Decimal,
        path: u16,
    ) -> crate::Result<Signature> {
        let (recipe, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
            &[b"recipe", collection.pubkey().as_ref()],
            &mpl_hybrid::ID,
        );

        let fee_ata =
            spl_associated_token_account::get_associated_token_address(fee_location, token);

        let ix = InitRecipeBuilder::new()
            .recipe(recipe)
            .authority(payer.pubkey())
            .collection(collection.pubkey())
            .token(*token)
            .fee_location(*fee_location)
            .fee_ata(fee_ata)
            .name(name)
            .uri(uri)
            .max(max)
            .min(min)
            .amount(ui_amount_to_amount(amount, fee_token_decimals).unwrap())
            .fee_amount_capture(
                ui_amount_to_amount(fee_amount_capture, fee_token_decimals).unwrap(),
            )
            .fee_amount_release(
                ui_amount_to_amount(fee_amount_release, fee_token_decimals).unwrap(),
            )
            .sol_fee_amount_capture(
                ui_amount_to_amount(sol_fee_amount_capture, fee_token_decimals).unwrap(),
            )
            .sol_fee_amount_release(
                ui_amount_to_amount(sol_fee_amount_release, fee_token_decimals).unwrap(),
            )
            .path(path)
            .associated_token_program(spl_associated_token_account::id())
            .instruction();

        let (mut init_recipe_tx, recent_blockhash) =
            execute(&ctx.solana_client, &payer.pubkey(), &[ix])
                .await
                .unwrap();

        init_recipe_tx
            .try_sign(&[payer.keypair().unwrap()], recent_blockhash)
            .unwrap();

        submit_transaction(&ctx.solana_client, init_recipe_tx).await
    }

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_run() {
        let ctx = Context::default();

        let fee_payer = Wallet::Keypair(Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ"));
        let fee_token_decimals = 9_u8;
        let collection = Wallet::Keypair(Keypair::new());
        let token = solana_sdk::pubkey!("AdaQ1MKbeKDyXCSnuCtqs5MW9FaY1UMGtpCGbZnpbTbj");
        let fee_location = Wallet::Keypair(Keypair::new());
        let name = String::from("collection name");
        let uri = String::from("https://example.com");
        let max = 1_u64;
        let min = 0_u64;
        let amount = rust_decimal_macros::dec!(0);
        let fee_amount_capture = rust_decimal_macros::dec!(0);
        let fee_amount_release = rust_decimal_macros::dec!(0);
        let sol_fee_amount_capture = rust_decimal_macros::dec!(0);
        let sol_fee_amount_release = rust_decimal_macros::dec!(0);
        let path = 1_u16;

        let create_collection_signature =
            create_collection(&ctx, &collection, &fee_payer, name.clone(), uri.clone())
                .await
                .unwrap();

        dbg!(create_collection_signature);

        let init_recipe_signature = init_recipe(
            &ctx,
            fee_token_decimals,
            &collection,
            &token,
            &fee_location.pubkey(),
            &fee_payer,
            name.clone(),
            uri.clone(),
            max,
            min,
            amount,
            fee_amount_capture,
            fee_amount_release,
            sol_fee_amount_capture,
            sol_fee_amount_release,
            path,
        )
        .await
        .unwrap();

        dbg!(init_recipe_signature);

        let output = run(
            ctx,
            super::Input {
                fee_payer: fee_payer.clone(),
                fee_token_decimals,
                authority: fee_payer.clone(),
                collection: collection.pubkey(),
                token,
                fee_location: fee_location.pubkey(),
                name: Some(name),
                uri: Some(uri),
                max: Some(1_u64),
                min: Some(0_u64),
                amount: Some(amount),
                fee_amount_capture: Some(fee_amount_capture),
                fee_amount_release: Some(fee_amount_release),
                sol_fee_amount_capture: Some(sol_fee_amount_capture),
                sol_fee_amount_release: Some(sol_fee_amount_release),
                path: Some(path),
                submit: true,
            },
        )
        .await
        .unwrap();

        dbg!(output.signature.unwrap());
    }
}
