use crate::prelude::*;

const NAME: &str = "get_balance";

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

fn build() -> BuildResult {
    const DEFINITION: &str = flow_lib::node_definition!("get_balance.json");
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

#[derive(Deserialize, Debug)]
pub struct Input {
    #[serde(with = "value::pubkey")]
    pubkey: Pubkey,
}

#[derive(Serialize, Debug)]
pub struct Output {
    balance: u64,
}

async fn run(ctx: Context, input: Input) -> Result<Output, CommandError> {
    let balance = ctx.solana_client.get_balance(&input.pubkey).await?;
    Ok(Output { balance })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_valid() {
        let cmd = build().unwrap();
        let ctx = Context::default();
        dbg!(ctx.solana_client.url());
        let output = cmd
            .run(
                ctx,
                value::map! { "pubkey" => Pubkey::new_from_array([1;32]) },
            )
            .await
            .unwrap();
        dbg!(output);
    }
}
