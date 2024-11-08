use flow_lib::command::prelude::*;
use mpl_hybrid::instructions::ReleaseV1Builder;

const NAME: &str = "release_v1";
flow_lib::submit!(CommandDescription::new(NAME, |_| build()));
fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_hybrid/release_v1.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    #[serde_as(as = "AsKeypair")]
    payer: Keypair,
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
    #[serde_as(as = "AsPubkey")]
    recent_blockhashes: Pubkey,
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

    let release_ins = ReleaseV1Builder::new()
        .owner(input.owner.pubkey())
        .authority(input.authority.pubkey())
        .escrow(input.escrow)
        .asset(input.asset)
        .collection(input.collection)
        .user_token_account(input.user_token_account)
        .escrow_token_account(input.escrow_token_account)
        .token(input.token)
        .fee_token_account(input.fee_token_account)
        .recent_blockhashes(input.recent_blockhashes)
        .mpl_core(input.mpl_core)
        .system_program(input.system_program)
        .token_program(input.token_program)
        .associated_token_account(input.associated_token_account);

    let ins = Instruction {
        fee_payer: input.payer.pubkey(),
        signers: [
            input.payer.clone_keypair(),
            input.owner.clone_keypair(),
            input.authority.clone_keypair,
        ]
        .into(),
        instructions: [release_ins].into(),
    };
    let signature = ctx
        .execute(ins, value::map! {})
        .await?
        .signature;
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
        build()
            .unwrap()
            .run(ctx, ValueSet::new())
            .await
            .unwrap_err();
    }
}
