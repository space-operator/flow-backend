use crate::{prelude::*, utils::ui_amount_to_amount};
use flow_lib::command::prelude::*;
use mpl_hybrid::instructions::UpdateEscrowV1Builder;

const NAME: &str = "update_escrow";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_404/update_escrow.json");
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

    let update_escrow_ix = UpdateEscrowV1Builder::new()
        .escrow(input.escrow)
        .authority(input.authority.pubkey())
        .collection(input.collection)
        .token(input.token)
        .fee_location(input.fee_location)
        .instruction();

    let ix = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.authority.clone_keypair(),
        ]
        .into(),
        instructions: [update_escrow_ix].into(),
    };

    let signature = ctx.execute(ix, value::map! {}).await?.signature;

    Ok(Output { signature })
}
