use flow_lib::command::prelude::*;
use mpl_hybrid::instructions::InitNftDataV1Builder;

const NAME: &str = "init_nft_data_v1";
flow_lib::submit!(CommandDescription::new(NAME, |_| build()));
fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_hybrid/init_nft_data_v1.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Input {
    #[serde_as(as = "AsKeypair")]
    payer: Keypair,
    #[serde_as(as = "AsPubkey")]
    nft_data: Pubkey,
    #[serde_as(as = "AsKeypair")]
    authority: Keypair,
    #[serde_as(as = "AsPubkey")]
    asset: Pubkey,
    #[serde_as(as = "AsPubkey")]
    collection: Pubkey,
    #[serde_as(as = "AsPubkey")]
    token: Pubkey,
    #[serde_as(as = "AsPubkey")]
    fee_location: Pubkey,
    #[serde_as(as = "Option<AsPubkey>")]
    system_program: Option<Pubkey>,
    name: String,
    uri: String,
    max: u64,
    min: u64,
    fee_amount: u64,
    sol_fee_amount: u64,
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

    let init_nft_data_ins = InitEscrowV1Builder::new()
        .nft_data(input.nft_data)
        .authority(input.authority.pubkey())
        .asset(input.asset)
        .collection(input.collection)
        .token(input.token)
        .fee_location(input.fee_location)
        .system_program(input.system_program)
        .name(input.name)
        .ure(input.ure)
        .max(input.max)
        .min(input.min)
        .amount(input.amount)
        .fee_amount(input.fee_amount)
        .sol_fee_amount(input.sol_fee_amount)
        .path(input.path);

    let ins = Instruction {
        fee_payer: input.payer.pubkey(),
        signers: [input.payer.clone_keypair(), input.authority.clone_keypair()].into(),
        instructions: [init_nft_data_ins].into(),
    };

    let signature = ctx.execute(ins, value::map! {}).await?.signature;
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
