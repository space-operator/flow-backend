use crate::prelude::*;
use super::parse_swig_account;

const NAME: &str = "swig_get_account";
const DEFINITION: &str = flow_lib::node_definition!("swig/swig_get_account.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde_as(as = "AsPubkey")]
    pub swig_account: Pubkey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let rpc = ctx.solana_client();
    let account = rpc
        .get_account(&input.swig_account)
        .await
        .map_err(|e| CommandError::msg(format!("Failed to fetch Swig account: {e}")))?;

    let result = parse_swig_account(&account.data)?;
    Ok(Output { result })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
