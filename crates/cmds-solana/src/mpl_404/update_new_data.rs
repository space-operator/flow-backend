use crate::{prelude::*, utils::ui_amount_to_amount};
use flow_lib::command::prelude::*;
use mpl_hybrid::instructions::UpdateNewDataV1Builder;

const NAME: &str = "update_new_data";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_404/update_new_data.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    fee_payer: Wallet,
    fee_token_decimals: u8,

    authority: Wallet,
    #[serde_as(as = "AsPubkey")]
    collection: Pubkey,
    #[serde_as(as = "AsPubkey")]
    asset: Pubkey,
    #[serde_as(as = "AsPubkey")]
    token: Pubkey,
    #[serde_as(as = "AsPubkey")]
    fee_location: Pubkey,
    name: Option<String>,
    uri: Option<String>,
    max: Option<u64>,
    min: Option<u64>,
    #[serde_as(as = "Option<AsDecimal>")]
    amount: Option<Decimal>,
    #[serde_as(as = "Option<AsDecimal>")]
    fee_amount: Option<Decimal>,
    #[serde_as(as = "Option<AsDecimal>")]
    sol_fee_amount: Option<Decimal>,
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
    #[serde_as(as = "Option<AsSignature>")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    tracing::info!("input: {:?}", input);

    let (nft_data, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"nft", input.asset.as_ref()],
        &mpl_hybrid::ID,
    );

    let sol_fee_decimals = 9;

    let mut builder = UpdateNewDataV1Builder::new();
    let mut update_new_data_ix = builder
        .nft_data(nft_data)
        .authority(input.authority.pubkey())
        .collection(input.collection)
        .asset(input.asset)
        .token(input.token)
        .fee_location(input.fee_location);

    if let Some(name) = input.name {
        update_new_data_ix = update_new_data_ix.name(name);
    }

    if let Some(uri) = input.uri {
        update_new_data_ix = update_new_data_ix.uri(uri);
    }

    if let Some(max) = input.max {
        update_new_data_ix = update_new_data_ix.max(max);
    }

    if let Some(min) = input.min {
        update_new_data_ix = update_new_data_ix.min(min);
    }

    if let Some(amount) = input.amount {
        update_new_data_ix =
            update_new_data_ix.amount(ui_amount_to_amount(amount, input.fee_token_decimals)?);
    }

    if let Some(fee_amount) = input.fee_amount {
        update_new_data_ix = update_new_data_ix
            .fee_amount(ui_amount_to_amount(fee_amount, input.fee_token_decimals)?);
    }

    if let Some(sol_fee_amount) = input.sol_fee_amount {
        update_new_data_ix = update_new_data_ix
            .sol_fee_amount(ui_amount_to_amount(sol_fee_amount, sol_fee_decimals)?);
    }

    if let Some(path) = input.path {
        update_new_data_ix = update_new_data_ix.path(path);
    }

    let ix = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: [update_new_data_ix.instruction()].into(),
    };

    let signature = ctx.execute(ix, value::map! {}).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use mpl_core::instructions::{CreateCollectionV2Builder, CreateV1Builder};
    use mpl_hybrid::instructions::{InitEscrowV1Builder, InitNftDataV1Builder};

    use crate::utils::ui_amount_to_amount;

    use super::*;

    async fn create_collection(
        ctx: &Context,
        collection: &Wallet,
        payer: &Wallet,
        name: String,
        uri: String,
    ) -> crate::Result<Signature> {
        let ix = CreateCollectionV2Builder::new()
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

    async fn create_asset(
        ctx: &Context,
        payer: &Wallet,
        collection: &Wallet,
        owner: &Pubkey,
        name: String,
        uri: String,
    ) -> crate::Result<(Pubkey, Signature)> {
        let asset = Wallet::Keypair(Keypair::new());

        let ix = CreateV1Builder::new()
            .payer(payer.pubkey())
            .asset(asset.pubkey())
            .collection(Some(collection.pubkey()))
            .owner(Some(*owner))
            .name(name)
            .uri(uri)
            .instruction();

        let (mut create_asset_tx, recent_blockhash) =
            execute(&ctx.solana_client, &payer.pubkey(), &[ix])
                .await
                .unwrap();

        create_asset_tx
            .try_sign(
                &[asset.keypair().unwrap(), payer.keypair().unwrap()],
                recent_blockhash,
            )
            .unwrap();

        let result_signature = submit_transaction(&ctx.solana_client, create_asset_tx).await;
        result_signature.map(|signature| (asset.pubkey(), signature))
    }

    async fn init_escrow(
        ctx: &Context,
        payer: &Wallet,
        collection: &Wallet,
        token: &Pubkey,
        fee_wallet: &Pubkey,
        escrow_name: String,
        uri: String,
        max: u64,
        min: u64,
        path: u16,
    ) -> crate::Result<(Pubkey, Signature)> {
        let (escrow, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
            &[b"escrow", collection.pubkey().as_ref()],
            &mpl_hybrid::ID,
        );

        let fee_ata = spl_associated_token_account::get_associated_token_address(fee_wallet, token);

        let fee_token_decimals = 9;
        let sol_fee_decimals = 9;

        let ix = InitEscrowV1Builder::new()
            .escrow(escrow)
            .authority(payer.pubkey())
            .collection(collection.pubkey())
            .token(*token)
            .fee_location(*fee_wallet)
            .fee_ata(fee_ata)
            .name(escrow_name)
            .uri(uri)
            .max(max)
            .min(min)
            .amount(ui_amount_to_amount(
                rust_decimal_macros::dec!(0),
                fee_token_decimals,
            )?)
            .fee_amount(ui_amount_to_amount(
                rust_decimal_macros::dec!(0),
                fee_token_decimals,
            )?)
            .sol_fee_amount(ui_amount_to_amount(
                rust_decimal_macros::dec!(0),
                sol_fee_decimals,
            )?)
            .path(path)
            .instruction();

        let (mut init_escrow_tx, recent_blockhash) =
            execute(&ctx.solana_client, &payer.pubkey(), &[ix])
                .await
                .unwrap();

        init_escrow_tx
            .try_sign(
                &[payer.keypair().unwrap(), payer.keypair().unwrap()],
                recent_blockhash,
            )
            .unwrap();

        let result_signature = submit_transaction(&ctx.solana_client, init_escrow_tx).await;
        result_signature.map(|signature| (escrow, signature))
    }

    async fn init_nft_data(
        ctx: &Context,
        payer: &Wallet,
        asset: &Pubkey,
        collection: &Wallet,
        token: &Pubkey,
        fee_wallet: &Pubkey,
        name: String,
        uri: String,
        max: u64,
        min: u64,
        path: u16,
        fee_token_decimals: u8,
        amount: Decimal,
        fee_amount: Decimal,
        sol_fee_amount: Decimal,
    ) -> crate::Result<Signature> {
        let (nft_data, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
            &[b"nft", asset.as_ref()],
            &mpl_hybrid::ID,
        );

        let sol_fee_decimals = 9;

        let ix = InitNftDataV1Builder::new()
            .nft_data(nft_data)
            .authority(payer.pubkey())
            .asset(*asset)
            .collection(collection.pubkey())
            .token(*token)
            .fee_location(*fee_wallet)
            .name(name)
            .uri(uri)
            .max(max)
            .min(min)
            .amount(ui_amount_to_amount(amount, fee_token_decimals)?)
            .fee_amount(ui_amount_to_amount(fee_amount, fee_token_decimals)?)
            .sol_fee_amount(ui_amount_to_amount(sol_fee_amount, sol_fee_decimals)?)
            .path(path)
            .instruction();

        let (mut init_nft_data_tx, recent_blockhash) =
            execute(&ctx.solana_client, &payer.pubkey(), &[ix])
                .await
                .unwrap();

        init_nft_data_tx
            .try_sign(&[payer.keypair().unwrap()], recent_blockhash)
            .unwrap();

        submit_transaction(&ctx.solana_client, init_nft_data_tx).await
    }

    async fn setup(
        ctx: &Context,
        payer: &Wallet,
        collection: &Wallet,
        token: &Pubkey,
        fee_wallet: &Pubkey,
        escrow_name: String,
        name: String,
        uri: String,
        max: u64,
        min: u64,
        path: u16,
        fee_token_decimals: u8,
        amount: Decimal,
        fee_amount: Decimal,
        sol_fee_amount: Decimal,
    ) -> (Pubkey, Pubkey) {
        // create collection
        let create_collection_signature = create_collection(
            ctx,
            collection,
            payer,
            String::from("mock_collection"),
            String::from("https://example.com"),
        )
        .await
        .unwrap();

        dbg!(create_collection_signature);

        // init escrow
        let (escrow, init_escrow_signature) = init_escrow(
            ctx,
            payer,
            collection,
            token,
            fee_wallet,
            escrow_name,
            uri.clone(),
            max,
            min,
            path,
        )
        .await
        .unwrap();

        dbg!(init_escrow_signature);

        // create asset
        let (asset, create_asset_signature) = create_asset(
            ctx,
            payer,
            collection,
            &escrow,
            String::from("mock_asset_nft"),
            String::from("https://example.com/0.json"),
        )
        .await
        .unwrap();

        dbg!(create_asset_signature);

        // init nft data
        init_nft_data(
            ctx,
            payer,
            &asset,
            collection,
            token,
            fee_wallet,
            name.clone(),
            uri.clone(),
            max,
            min,
            path,
            fee_token_decimals,
            amount,
            fee_amount,
            sol_fee_amount,
        )
        .await
        .unwrap();

        (escrow, asset)
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
        let fee_wallet = Wallet::Keypair(Keypair::new());
        let escrow_name = String::from("Escrow Name");
        let name = String::from("mock_nft_name");
        let uri = String::from("https://base.spaceoperator.com/storage/v1/object/public/blings_gg_nft/asset_metadata.json");
        let max = 1_u64;
        let min = 0_u64;
        let amount = rust_decimal_macros::dec!(0);
        let fee_amount = rust_decimal_macros::dec!(0);
        let sol_fee_amount = rust_decimal_macros::dec!(0);
        let path = 1;

        let (_escrow, asset) = setup(
            &ctx,
            &fee_payer,
            &collection,
            &token,
            &fee_wallet.pubkey(),
            escrow_name.clone(),
            name.clone(),
            uri.clone(),
            max,
            min,
            path,
            fee_token_decimals,
            amount,
            fee_amount,
            sol_fee_amount,
        )
        .await;

        let output = run(
            ctx,
            super::Input {
                fee_payer: fee_payer.clone(),
                fee_token_decimals,
                authority: fee_payer.clone(),
                collection: collection.pubkey(),
                token,
                asset,
                fee_location: fee_wallet.pubkey(),
                name: Some(name),
                uri: Some(uri),
                max: Some(max),
                min: Some(min),
                amount: Some(amount),
                fee_amount: Some(fee_amount),
                sol_fee_amount: Some(sol_fee_amount),
                path: Some(path),
                submit: true,
            },
        )
        .await
        .unwrap();

        dbg!(output.signature.unwrap());
    }
}
