use crate::prelude::*;
use super::find_wallet_address;

const NAME: &str = "swig_find_wallet_address";
const DEFINITION: &str = flow_lib::node_definition!("swig/swig_find_wallet_address.jsonc");

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

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde_as(as = "AsPubkey")]
    pub wallet_address: Pubkey,
    pub bump: u8,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (wallet_address, bump) = find_wallet_address(&input.swig_account);
    Ok(Output { wallet_address, bump })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
