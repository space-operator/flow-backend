use flow_lib::command::prelude::*;
const NAME: &str = "mpl_hybrid";
flow_lib::submit!(CommandDescription::new(NAME, | _ | build()));
fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_hybrid/mpl_hybrid.json");
    static CACHE: BuilderCache = BuilderCache::new(|| {
        CmdBuilder::new(DEFINITION)?.check_name(NAME)
    });
    Ok(CACHE.clone()?.build(run))
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    #[serde(with = "value::keypair")]
    pub fee_payer: Keypair,
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
    #[serde_as(as = "AsPubkey")]
    fee_sol_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    fee_project_account: Pubkey,
    #[serde_as(as = "AsPubkey")]
    recent_blockhashes: Pubkey,
    #[serde_as(as = "AsPubkey")]
    mpl_core: Pubkey,
    #[serde_as(as = "AsPubkey")]
    system_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    token_program: Pubkey,
    #[serde_as(as = "AsPubkey")]
    associated_token_program: Pubkey,
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
    let recent_blockhash = client.get_latest_blockhash().await?;
    let blockhash_as_str = recent_blockhash.clone().to_string();
    let capture_instraction = mpl_hybrid::instructions::CaptureV1{
        input.owner,
        input.authority,
        input.escrow,
        input.asset,
        input.collection,
        input.user_token_account,
        input.escrow_token_account,
        input.token,
        input.fee_token_account,
        input.fee_sol_account,
        input.fee_project_account,
        input.fee_token_account,
        input.recent_blockhashes,
        input.mpl_core,
        input.system_program,
        input.token_program,
        input.associated_token_program,
    }.instruction_with_remaining_accounts(&[]);

    let instructions = Instructions {
        fee_payer: input.fee_payer.pubkey(),
        signers: [input.fee_payer.clone_keypair()].into(),
        instructions: [capture_instraction].into(),
    };
    let signature = ctx.execute(instructions, <_>::default()).await?.signature;

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
    async fn test_run() {
        let ctx = Context::default();
        build().unwrap().run(ctx, ValueSet::new()).await.unwrap_err();
    }
}
