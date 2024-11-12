use crate::{prelude::*, utils::ui_amount_to_amount};
use flow_lib::command::prelude::*;
use mpl_hybrid::instructions::InitRecipeBuilder;

const NAME: &str = "init_recipe";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_404/init_recipe.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    #[serde_as(as = "AsKeypair")]
    fee_payer: Keypair,
    fee_token_decimals: u8,

    // accounts
    #[serde_as(as = "AsKeypair")]
    authority: Keypair,
    #[serde_as(as = "AsPubkey")]
    collection: Pubkey,
    #[serde_as(as = "AsPubkey")]
    token: Pubkey,
    #[serde_as(as = "AsPubkey")]
    fee_location: Pubkey,

    // args
    name: String,
    uri: String,
    max: u64,
    min: u64,
    #[serde_as(as = "AsDecimal")]
    amount: Decimal,
    #[serde_as(as = "AsDecimal")]
    fee_amount_capture: Decimal,
    #[serde_as(as = "AsDecimal")]
    fee_amount_release: Decimal,
    #[serde_as(as = "AsDecimal")]
    sol_fee_amount_capture: Decimal,
    #[serde_as(as = "AsDecimal")]
    sol_fee_amount_release: Decimal,
    // CURRENT PATH OPTIONS:
    // 0-- NFT METADATA IS UPDATED ON SWAP
    // 1-- NFT METADATA IS NOT UPDATED ON SWAP
    path: u16,

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

    let sol_token_decimals = 9;

    let (recipe, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"recipe", input.collection.as_ref()],
        &mpl_hybrid::ID,
    );

    let fee_ata = spl_associated_token_account::get_associated_token_address(
        &input.fee_location,
        &input.token,
    );

    let init_recipe_ix = InitRecipeBuilder::new()
        .recipe(recipe)
        .authority(input.authority.pubkey())
        .collection(input.collection)
        .token(input.token)
        .fee_location(input.fee_location)
        .fee_ata(fee_ata)
        .name(input.name)
        .uri(input.uri)
        .max(input.max)
        .min(input.min)
        .amount(ui_amount_to_amount(input.amount, input.fee_token_decimals)?)
        .fee_amount_capture(ui_amount_to_amount(
            input.fee_amount_capture,
            input.fee_token_decimals,
        )?)
        .fee_amount_release(ui_amount_to_amount(
            input.fee_amount_release,
            input.fee_token_decimals,
        )?)
        .sol_fee_amount_capture(ui_amount_to_amount(
            input.sol_fee_amount_capture,
            sol_token_decimals,
        )?)
        .sol_fee_amount_release(ui_amount_to_amount(
            input.sol_fee_amount_release,
            sol_token_decimals,
        )?)
        .path(input.path)
        .associated_token_program(spl_associated_token_account::id())
        .instruction();

    let ix = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.authority.clone_keypair(),
        ]
        .into(),
        instructions: [init_recipe_ix].into(),
    };

    let signature = ctx.execute(ix, value::map! {}).await?.signature;

    Ok(Output { recipe, signature })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_collection(
        ctx: &Context,
        collection: &Keypair,
        payer: &Keypair,
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

        try_sign_wallet(
            ctx,
            &mut create_collection_tx,
            &[payer, collection],
            recent_blockhash,
        )
        .await
        .unwrap();

        submit_transaction(&ctx.solana_client, create_collection_tx).await
    }

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_run() {
        let ctx = Context::default();

        let fee_payer = Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ");
        let fee_token_decimals = 9_u8;
        let collection = Keypair::new();
        let token = solana_sdk::pubkey!("AdaQ1MKbeKDyXCSnuCtqs5MW9FaY1UMGtpCGbZnpbTbj");
        let fee_location = Keypair::new().pubkey();
        let name = String::from("collection name");
        let uri = String::from("https://example.com");

        let create_collection_signature =
            create_collection(&ctx, &collection, &fee_payer, name.clone(), uri.clone())
                .await
                .unwrap();

        dbg!(create_collection_signature);

        let output = run(
            ctx,
            super::Input {
                fee_payer: fee_payer.clone_keypair(),
                fee_token_decimals,
                authority: fee_payer.clone_keypair(),
                collection: collection.pubkey(),
                token,
                fee_location,
                name,
                uri,
                max: 1_u64,
                min: 0_u64,
                amount: rust_decimal_macros::dec!(0),
                fee_amount_capture: rust_decimal_macros::dec!(0),
                fee_amount_release: rust_decimal_macros::dec!(0),
                sol_fee_amount_capture: rust_decimal_macros::dec!(0),
                sol_fee_amount_release: rust_decimal_macros::dec!(0),
                path: 1_u16,
                submit: true,
            },
        )
        .await
        .unwrap();

        dbg!(output.signature.unwrap());
    }
}
