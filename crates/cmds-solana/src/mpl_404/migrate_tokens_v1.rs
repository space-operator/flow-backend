use crate::{prelude::*, utils::ui_amount_to_amount};
use flow_lib::command::prelude::*;
use mpl_hybrid::instructions::MigrateTokensV1Builder;

const NAME: &str = "migrate_tokens_v1";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_404/migrate_tokens_v1.json");
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

    // args
    #[serde_as(as = "AsDecimal")]
    amount: Decimal,

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

    // escrow new
    let (escrow_new, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"escrow", input.authority.pubkey().as_ref()],
        &mpl_hybrid::ID,
    );

    let escrow_new_token_account =
        spl_associated_token_account::get_associated_token_address(&escrow_new, &input.token);

    // escrow old
    let (escrow_old, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"escrow", input.collection.as_ref()],
        &mpl_hybrid::ID,
    );

    let escrow_old_token_account =
        spl_associated_token_account::get_associated_token_address(&escrow_old, &input.token);

    let migrate_tokens_ix = MigrateTokensV1Builder::new()
        .authority(input.authority.pubkey())
        .escrow_new(escrow_new)
        .escrow_old(escrow_old)
        .collection(input.collection)
        .escrow_new_token_account(escrow_new_token_account)
        .escrow_old_token_account(escrow_old_token_account)
        .token(input.token)
        .associated_token_program(spl_associated_token_account::ID)
        .amount(ui_amount_to_amount(input.amount, input.fee_token_decimals).unwrap())
        .instruction();

    let ix = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer, input.authority].into(),
        instructions: [migrate_tokens_ix].into(),
    };

    let signature = ctx.execute(ix, value::map! {}).await?.signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use mpl_core::instructions::{CreateCollectionV2Builder, CreateV1Builder};
    use mpl_hybrid::instructions::{InitEscrowV1Builder, InitEscrowV2Builder};

    use crate::utils::ui_amount_to_amount;

    use super::*;

    async fn transfer_sol(
        ctx: &Context,
        from_pubkey: &Wallet,
        to_pubkey: &Pubkey,
        amount: Decimal,
    ) -> crate::Result<Signature> {
        let ix = solana_sdk::system_instruction::transfer(
            &from_pubkey.pubkey(),
            to_pubkey,
            ui_amount_to_amount(amount, 9).unwrap(),
        );

        let (mut transfer_sol_tx, recent_blockhash) =
            execute(&ctx.solana_client, &from_pubkey.pubkey(), &[ix])
                .await
                .unwrap();

        transfer_sol_tx
            .try_sign(&[&from_pubkey.keypair().unwrap()], recent_blockhash)
            .unwrap();

        submit_transaction(&ctx.solana_client, transfer_sol_tx).await
    }

    async fn create_collection(
        ctx: &Context,
        collection: Wallet,
        payer: Wallet,
        authority: Wallet,
        name: String,
        uri: String,
    ) -> crate::Result<Signature> {
        let ix = CreateCollectionV2Builder::new()
            .collection(collection.pubkey())
            .update_authority(Some(authority.pubkey()))
            .payer(payer.pubkey())
            .name(name)
            .uri(uri)
            .instruction();

        let (mut create_collection_tx, recent_blockhash) =
            execute(&ctx.solana_client, &payer.pubkey(), &[ix])
                .await
                .unwrap();

        create_collection_tx
            .try_sign(
                &[&payer.keypair().unwrap(), &collection.keypair().unwrap()],
                recent_blockhash,
            )
            .unwrap();

        submit_transaction(&ctx.solana_client, create_collection_tx).await
    }

    async fn create_asset(
        ctx: &Context,
        payer: Wallet,
        authority: Wallet,
        collection: Wallet,
        owner: &Pubkey,
        name: String,
        uri: String,
    ) -> crate::Result<(Pubkey, Signature)> {
        let asset = Wallet::Keypair(Keypair::new());

        let ix = CreateV1Builder::new()
            .payer(payer.pubkey())
            .authority(Some(authority.pubkey()))
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
                &[
                    &asset.keypair().unwrap(),
                    &asset.keypair().unwrap(),
                    &payer.keypair().unwrap(),
                    &authority.keypair().unwrap(),
                ],
                recent_blockhash,
            )
            .unwrap();

        let result_signature = submit_transaction(&ctx.solana_client, create_asset_tx).await;
        result_signature.map(|signature| (asset.pubkey(), signature))
    }

    async fn init_escrow_v1(
        ctx: &Context,
        payer: &Wallet,
        authority: &Wallet,
        collection: &Wallet,
        token: Pubkey,
        fee_wallet: Pubkey,
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

        let fee_ata =
            spl_associated_token_account::get_associated_token_address(&fee_wallet, &token);

        let fee_token_decimals = 9;
        let sol_fee_decimals = 9;

        let ix = InitEscrowV1Builder::new()
            .escrow(escrow)
            .authority(authority.pubkey())
            .collection(collection.pubkey())
            .token(token)
            .fee_location(fee_wallet)
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

        let create_escrow_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &payer.pubkey(),
                &escrow,
                &token,
                &spl_token::id(),
            );

        let (mut init_escrow_v1_tx, recent_blockhash) = execute(
            &ctx.solana_client,
            &payer.pubkey(),
            &[ix, create_escrow_ata_ix],
        )
        .await
        .unwrap();

        init_escrow_v1_tx
            .try_sign(
                &[&payer.keypair().unwrap(), &authority.keypair().unwrap()],
                recent_blockhash,
            )
            .unwrap();

        let result_signature = submit_transaction(&ctx.solana_client, init_escrow_v1_tx).await;
        result_signature.map(|signature| (escrow, signature))
    }

    async fn init_escrow_v2(
        ctx: &Context,
        fee_payer: Wallet,
        authority: Wallet,
    ) -> crate::Result<(Pubkey, Signature)> {
        let (escrow, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
            &[b"escrow", authority.pubkey().as_ref()],
            &mpl_hybrid::ID,
        );

        let ix = InitEscrowV2Builder::new()
            .escrow(escrow)
            .authority(authority.pubkey())
            .instruction();

        let (mut init_escrow_v2_tx, recent_blockhash) =
            execute(&ctx.solana_client, &fee_payer.pubkey(), &[ix])
                .await
                .unwrap();

        init_escrow_v2_tx
            .try_sign(
                &[&fee_payer.keypair().unwrap(), &authority.keypair().unwrap()],
                recent_blockhash,
            )
            .unwrap();

        let result_signature = submit_transaction(&ctx.solana_client, init_escrow_v2_tx).await;
        result_signature.map(|signature| (escrow, signature))
    }

    async fn setup(
        ctx: &Context,
        payer: &Wallet,
        authority: &Wallet,
        collection: &Wallet,
        token: Pubkey,
        fee_wallet: Pubkey,
        escrow_name: String,
        uri: String,
        max: u64,
        min: u64,
        path: u16,
    ) -> (Pubkey, Pubkey, Pubkey) {
        // transfer sol to authority
        let transfer_sol_signature = transfer_sol(
            ctx,
            &payer,
            &authority.pubkey(),
            rust_decimal_macros::dec!(0.03),
        )
        .await
        .unwrap();

        dbg!(transfer_sol_signature);

        // create collection
        let create_collection_signature = create_collection(
            ctx,
            collection.clone(),
            payer.clone(),
            authority.clone(),
            String::from("mock_collection"),
            String::from("https://example.com"),
        )
        .await
        .unwrap();

        dbg!(create_collection_signature);

        // init escrow
        let (escrow_v1, init_escrow_v1_signature) = init_escrow_v1(
            ctx,
            &payer,
            &authority,
            &collection,
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

        dbg!(init_escrow_v1_signature);

        // init_escrow_v2
        let (escrow_v2, init_escrow_v2_signature) =
            init_escrow_v2(ctx, payer.clone(), authority.clone())
                .await
                .unwrap();

        dbg!(init_escrow_v2_signature);

        // create asset
        let (asset, create_asset_signature) = create_asset(
            ctx,
            payer.clone(),
            authority.clone(),
            collection.clone(),
            &escrow_v1,
            String::from("mock_asset_nft"),
            String::from("https://example.com/0.json"),
        )
        .await
        .unwrap();

        dbg!(create_asset_signature);

        (escrow_v1, escrow_v2, asset)
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
        let fee_token_decimals = 9_u8;
        let authority = Wallet::Keypair(Keypair::new());
        let collection = Wallet::Keypair(Keypair::new());
        let token = solana_sdk::pubkey!("AdaQ1MKbeKDyXCSnuCtqs5MW9FaY1UMGtpCGbZnpbTbj");
        let fee_wallet = Wallet::Keypair(Keypair::new());
        let name = String::from("Escrow Name");
        let uri = String::from("https://base.spaceoperator.com/storage/v1/object/public/blings_gg_nft/asset_metadata.json");
        let max = 1_u64;
        let min = 0_u64;
        let path = 1;
        let amount = rust_decimal_macros::dec!(0);

        let (_escrow_v1, _escrow_v2, _asset) = setup(
            &ctx,
            &fee_payer,
            &authority,
            &collection,
            token,
            fee_wallet.pubkey(),
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
                fee_payer: fee_payer,
                fee_token_decimals,
                authority: authority,
                collection: collection.pubkey(),
                token,
                amount,
                submit: true,
            },
        )
        .await
        .unwrap();

        dbg!(output.signature.unwrap());
    }
}
