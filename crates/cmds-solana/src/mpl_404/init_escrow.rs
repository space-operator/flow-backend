use crate::{prelude::*, utils::ui_amount_to_amount};
use flow_lib::command::prelude::*;
use mpl_hybrid::instructions::InitEscrowV1Builder;

const NAME: &str = "init_escrow";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_404/init_escrow.json");
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
    token: Pubkey,
    #[serde_as(as = "AsPubkey")]
    fee_location: Pubkey,
    name: String,
    uri: String,
    max: u64,
    min: u64,
    #[serde_as(as = "AsDecimal")]
    amount: Decimal,
    #[serde_as(as = "AsDecimal")]
    fee_amount: Decimal,
    #[serde_as(as = "AsDecimal")]
    sol_fee_amount: Decimal,
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
    #[serde_as(as = "Option<AsSignature>")]
    pub signature: Option<Signature>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    tracing::info!("input: {:?}", input);

    let (escrow, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"escrow", input.collection.as_ref()],
        &mpl_hybrid::ID,
    );

    let fee_ata = spl_associated_token_account::get_associated_token_address(
        &input.fee_location,
        &input.token,
    );

    let sol_fee_decimals = 9;

    let init_escrow_ix = InitEscrowV1Builder::new()
        .escrow(escrow)
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
        .fee_amount(ui_amount_to_amount(
            input.fee_amount,
            input.fee_token_decimals,
        )?)
        .sol_fee_amount(ui_amount_to_amount(input.sol_fee_amount, sol_fee_decimals)?)
        .path(input.path)
        .instruction();

    let ix = Instructions {
        lookup_tables: None,
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone(), input.authority].into(),
        instructions: [init_escrow_ix].into(),
    };

    let ix = input.submit.then_some(ix).unwrap_or_default();

    let signature = ctx
        .execute(
            ix,
            value::map! {
                "escrow" => escrow,
            },
        )
        .await?
        .signature;

    Ok(Output { signature })
}

#[cfg(test)]
mod tests {
    use mpl_core::instructions::CreateCollectionV2Builder;
    use solana_sdk::native_token::LAMPORTS_PER_SOL;

    use super::*;

    async fn create_mock_collection(
        ctx: &Context,
        collection: Wallet,
        payer: Wallet,
        name: String,
        uri: String,
    ) {
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

        let create_collection_signature =
            submit_transaction(&ctx.solana_client, create_collection_tx)
                .await
                .unwrap();

        dbg!(create_collection_signature);
    }

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_run() {
        let ctx = Context::default();

        // setup fee_payer
        let fee_payer = Wallet::Keypair(Keypair::from_base58_string("4rQanLxTFvdgtLsGirizXejgYXACawB5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ"));
        let balance = ctx
            .solana_client
            .get_balance(&fee_payer.pubkey())
            .await
            .unwrap() as f64
            / LAMPORTS_PER_SOL as f64;

        if balance < 0.1 {
            let _ = ctx
                .solana_client
                .request_airdrop(&fee_payer.pubkey(), LAMPORTS_PER_SOL)
                .await;
        }

        let token = solana_sdk::pubkey!("AdaQ1MKbeKDyXCSnuCtqs5MW9FaY1UMGtpCGbZnpbTbj");
        let collection = Wallet::Keypair(Keypair::new());
        create_mock_collection(
            &ctx,
            collection.clone(),
            fee_payer.clone(),
            String::from("Mock Collection"),
            String::from("https://example.com"),
        )
        .await;

        // setup SUT
        let fee_token_decimals = 9_u8;
        let authority = fee_payer.clone();
        let fee_wallet = Wallet::Keypair(Keypair::new());
        let name = String::from("Escrow Name");
        let uri = String::from("https://base.spaceoperator.com/storage/v1/object/public/blings_gg_nft/asset_metadata.json");
        let max = 999_u64;
        let min = 0_u64;
        let path = 1;

        // init escrow
        let output = run(
            ctx,
            super::Input {
                fee_payer: fee_payer.clone(),
                fee_token_decimals,
                authority: authority.clone(),
                collection: collection.pubkey(),
                token,
                fee_location: fee_wallet.pubkey(),
                name,
                uri,
                max,
                min,
                amount: rust_decimal_macros::dec!(0.1),
                fee_amount: rust_decimal_macros::dec!(0.1),
                sol_fee_amount: rust_decimal_macros::dec!(0.1),
                path,
                submit: true,
            },
        )
        .await
        .unwrap();
        dbg!(output.signature.unwrap());
    }
}
