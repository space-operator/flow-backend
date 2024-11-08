use flow_lib::command::prelude::*;
use mpl_hybrid::instructions::CaptureV1Builder;

const NAME: &str = "capture_v1";
flow_lib::submit!(CommandDescription::new(NAME, |_| build()));
fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_hybrid/capture_v1.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    #[serde_as(as = "AsPubkey")]
    owner: Pubkey,
    #[serde_as(as = "AsPubkey")]
    authority: Pubkey,
    #[serde_as(as = "AsPubkey")]
    escrow: Pubkey,
    #[serde_as(as = "AsPubkey")]
    asset: Pubkey,
    #[serde_as(as = "AsPubkey")]
    collection: Pubkey,
    #[serde_as(as = "AsPubkey")]
    user_token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    escrow_token_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    token: Pubkey,
    #[serde_as(as = "AsPubkey")]
    fee_token_account: Pubkey,
    #[serde_as(as = "Option<AsPubkey>")]
    fee_sol_account: Option<Pubkey>,
    #[serde_as(as = "AsPubkey")]
    fee_project_account: Pubkey,
    #[serde_as(as = "Option<AsPubkey>")]
    recent_blockhashes: Option<Pubkey>,
    #[serde_as(as = "Option<AsPubkey>")]
    mpl_core: Option<Pubkey>,
    #[serde_as(as = "Option<AsPubkey>")]
    system_program: Option<Pubkey>,
    #[serde_as(as = "Option<AsPubkey>")]
    token_program: Option<Pubkey>,
    #[serde_as(as = "Option<AsPubkey>")]
    associated_token_program: Option<Pubkey>,
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

    let capture_ins = CaptureV1Builder::new()
        .owner(input.owner.pubkey())
        .authority(input.authority.pubkey())
        .escrow(input.escrow)
        .asset(input.asset)
        .collection(input.collection)
        .user_token_account(input.user_token_account)
        .escrow_token_account(input.escrow_token_account)
        .token(input.token)
        .fee_token_account(input.fee_token_account)
        .fee_sol_account(input.fee_sol_account)
        .fee_project_account(input.fee_project_account)
        .recent_blockhashes(input.recent_blockhashes)
        .mpl_core(input.mpl_core)
        .system_program(input.system_program)
        .token_program(input.token_program)
        .associated_token_program(input.associated_token_program);

    let ins = Instructions {
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer.clone_keypair(),
            input.owner.clone_keypair(),
            input.authority.clone_keypair(),
        ]
        .into(),
        instructions: [capture_ins].into(),
    };

    let ins = input.submit.then_some(ins).unwrap_or_default();

    let signature = ctx.execute(ins, value::map! {}).await?.signature;
    Ok(Output { signature })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_build() {
        build().unwrap();
    }
    #[tokio::test]
    #[ignore]
    async fn test_run() {
        tracing_subscriber::fmt::try_init().ok();
        let ctx = Context::default();
        build()
            .unwrap()
            .run(ctx, ValueSet::new())
            .await
            .unwrap_err();
    }
}
