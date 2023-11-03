use crate::prelude::*;

const NAME: &str = "request_airdrop";

inventory::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = include_str!("../../../node-definitions/solana/request_airdrop.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

fn default_amount() -> u64 {
    1000000000
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    pub pubkey: Pubkey,
    #[serde(default = "default_amount")]
    pub amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(with = "value::signature")]
    pub signature: Signature,
}

async fn run(ctx: Context, input: Input) -> Result<Output, CommandError> {
    let signature = ctx
        .solana_client
        .request_airdrop(&input.pubkey, input.amount)
        .await?;
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
    async fn test_valid() {
        let pubkey = solana_sdk::pubkey!("DKsvmM9hfNm4R94yB3VdYMZJk2ETv5hpcjuRmiwgiztY");
        let amount: u64 = 1_500_000_000;

        let result = run(Context::default(), Input { amount, pubkey }).await;
        dbg!(&result);
    }
}
