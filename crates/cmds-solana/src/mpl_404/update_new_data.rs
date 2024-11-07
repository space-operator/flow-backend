use crate::{prelude::*, utils::ui_amount_to_amount};
use flow_lib::command::prelude::*;
use mpl_hybrid::instructions::UpdateNewDataV1Builder;

const NAME: &str = "update_new_data";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("mpl_404/update_new_data.json");
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
    nft_data: Pubkey,
    #[serde_as(as = "AsKeypair")]
    authority: Keypair,
    #[serde_as(as = "AsPubkey")]
    collection: Pubkey,
    #[serde_as(as = "AsPubkey")]
    asset: Pubkey,
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

    let sol_fee_decimals = 9;

    let update_new_data_ix = UpdateNewDataV1Builder::new()
        .nft_data(input.nft_data)
        .authority(input.authority.pubkey())
        .collection(input.collection)
        .asset(input.asset)
        .token(input.token)
        .fee_location(input.fee_location)
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
        fee_payer: input.fee_payer.pubkey(),
        signers: [
            input.fee_payer.clone_keypair(),
            input.authority.clone_keypair(),
        ]
        .into(),
        instructions: [update_new_data_ix].into(),
    };

    let signature = ctx.execute(ix, value::map! {}).await?.signature;

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
        build()
            .unwrap()
            .run(ctx, ValueSet::new())
            .await
            .unwrap_err();
    }
}
