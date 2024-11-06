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
    fee_token_account: Option<Pubkey>,
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

    let mut capture_v1_builder = CaptureV1Builder::new();
    let mut capture_ix = capture_v1_builder
        .owner(input.owner.pubkey())
        .authority(input.authority.pubkey())
        .escrow(input.escrow)
        .asset(input.asset)
        .collection(input.collection)
        .user_token_account(user_token_account)
        .escrow_token_account(escrow_token_account)
        .fee_project_account(input.fee_project_account);

    if let Some(fee_token_account) = input.fee_token_account {
        capture_ix = capture_ix.fee_token_account(fee_token_account);
    }
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
