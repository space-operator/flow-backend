use crate::prelude::*;
use super::find_swig_pda;

const NAME: &str = "swig_find_pda";
const DEFINITION: &str = flow_lib::node_definition!("swig/swig_find_pda.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub swig_id: [u8; 32],
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde_as(as = "AsPubkey")]
    pub swig_account: Pubkey,
    pub bump: u8,
}

async fn run(_ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let (swig_account, bump) = find_swig_pda(&input.swig_id);
    Ok(Output { swig_account, bump })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
