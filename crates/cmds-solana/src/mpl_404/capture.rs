use crate::prelude::*;
use flow_lib::{command::prelude::*, solana::KeypairExt};
use mpl_hybrid::instructions::CaptureV1Builder;

const NAME: &str = "capture";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_404/capture.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    #[serde_as(as = "AsKeypair")]
    fee_payer: Keypair,

    #[serde_as(as = "AsKeypair")]
    owner: Keypair,
    #[serde_as(as = "AsKeypair")]
    authority: Keypair,
    #[serde_as(as = "AsPubkey")]
    escrow: Pubkey,
    #[serde_as(as = "AsPubkey")]
    asset: Pubkey,
    #[serde_as(as = "AsPubkey")]
    collection: Pubkey,
    #[serde_as(as = "AsPubkey")]
    token: Pubkey,
    #[serde_as(as = "AsPubkey")]
    fee_project_account: Pubkey,
    #[serde_as(as = "Option<AsPubkey>")]
    fee_sol_account: Option<Pubkey>,
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

    let user_token_account = spl_associated_token_account::get_associated_token_address(
        &input.owner.pubkey(),
        &input.token,
    );
    let escrow_token_account =
        spl_associated_token_account::get_associated_token_address(&input.escrow, &input.token);

    let fee_token_account = spl_associated_token_account::get_associated_token_address(
        &input.fee_project_account,
        &input.token,
    );

    let mut capture_v1_builder = CaptureV1Builder::new();
    let mut capture_ix = capture_v1_builder
        .owner(input.owner.pubkey())
        .authority(input.authority.pubkey())
        .escrow(input.escrow)
        .asset(input.asset)
        .collection(input.collection)
        .user_token_account(user_token_account)
        .escrow_token_account(escrow_token_account)
        .token(input.token)
        .fee_token_account(fee_token_account)
        .fee_project_account(input.fee_project_account);

    if let Some(fee_sol_account) = input.fee_sol_account {
        capture_ix = capture_ix.fee_sol_account(fee_sol_account);
    }

    let ix = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.owner.clone_keypair(),
            input.authority.clone_keypair(),
        ]
        .into(),
        instructions: [capture_ix.instruction()].into(),
    };

    let ix = input.submit.then_some(ix).unwrap_or_default();

    let signature = ctx.execute(ix, value::map! {}).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use mpl_core::instructions::{CreateCollectionV2Builder, CreateV1Builder};
    use mpl_hybrid::instructions::InitEscrowV1Builder;

    use crate::utils::ui_amount_to_amount;

    use super::*;

    async fn create_collection(
        ctx: &Context,
        collection: &Keypair,
        payer: &Keypair,
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

    async fn create_asset(
        ctx: &Context,
        payer: &Keypair,
        collection: &Keypair,
        owner: &Pubkey,
        name: String,
        uri: String,
    ) -> crate::Result<(Pubkey, Signature)> {
        let asset = Keypair::new();

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

        try_sign_wallet(
            ctx,
            &mut create_asset_tx,
            &[&asset, payer],
            recent_blockhash,
        )
        .await
        .unwrap();

        let result_signature = submit_transaction(&ctx.solana_client, create_asset_tx).await;
        result_signature.map(|signature| (asset.pubkey(), signature))
    }

    async fn init_escrow(
        ctx: &Context,
        payer: &Keypair,
        collection: &Keypair,
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

        try_sign_wallet(ctx, &mut init_escrow_tx, &[payer, payer], recent_blockhash)
            .await
            .unwrap();

        let result_signature = submit_transaction(&ctx.solana_client, init_escrow_tx).await;
        result_signature.map(|signature| (escrow, signature))
    }

    async fn setup(
        ctx: &Context,
        payer: &Keypair,
        collection: &Keypair,
        token: &Pubkey,
        fee_wallet: &Pubkey,
        escrow_name: String,
        uri: String,
        max: u64,
        min: u64,
        path: u16,
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
            uri,
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

        (escrow, asset)
    }

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_run() {
        let ctx = Context::default();

        // setup fee_payer
        let fee_payer = Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ");
        let collection = Keypair::new();
        let token = solana_sdk::pubkey!("AdaQ1MKbeKDyXCSnuCtqs5MW9FaY1UMGtpCGbZnpbTbj");
        let fee_wallet = Keypair::new().pubkey();
        let name = String::from("Escrow Name");
        let uri = String::from("https://base.spaceoperator.com/storage/v1/object/public/blings_gg_nft/asset_metadata.json");
        let max = 1_u64;
        let min = 0_u64;
        let path = 1;

        let (escrow, asset) = setup(
            &ctx,
            &fee_payer,
            &collection,
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
                fee_payer: fee_payer.clone_keypair(),
                owner: fee_payer.clone_keypair(),
                authority: fee_payer.clone_keypair(),
                escrow,
                asset,
                collection: collection.pubkey(),
                token,
                fee_project_account: fee_wallet,
                fee_sol_account: None,
                submit: true,
            },
        )
        .await
        .unwrap();

        dbg!(output.signature.unwrap());
    }
}
