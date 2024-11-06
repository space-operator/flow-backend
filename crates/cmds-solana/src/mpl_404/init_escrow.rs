use crate::{prelude::*, utils::ui_amount_to_amount};
use flow_lib::{command::prelude::*, solana::KeypairExt};
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
    #[serde_as(as = "AsKeypair")]
    fee_payer: Keypair,
    fee_token_decimals: u8,

    #[serde_as(as = "AsPubkey")]
    escrow: Pubkey,
    #[serde_as(as = "AsKeypair")]
    authority: Keypair,
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
    path: String,

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

    let fee_ata = spl_associated_token_account::get_associated_token_address(
        &input.fee_location,
        &input.token,
    );

    let sol_fee_decimals = 9;

    let init_escrow_ix = InitEscrowV1Builder::new()
        .escrow(input.escrow)
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
        .instruction();

    let ix = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.authority.clone_keypair(),
        ]
        .into(),
        instructions: [init_escrow_ix].into(),
    };

    let ix = input.submit.then_some(ix).unwrap_or_default();

    let signature = ctx.execute(ix, value::map! {}).await?.signature;

    Ok(Output { signature })
}
