use crate::prelude::*;
use super::helper::{check_response, reflect_get};

pub const NAME: &str = "get_exchange_rate";
const DEFINITION: &str = flow_lib::node_definition!("reflect/get_exchange_rate.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub stablecoin_index: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = format!("/stablecoin/{}/exchange-rate", input.stablecoin_index);
    let req = reflect_get(&ctx, &path);
    let result = check_response(req.send().await?).await?;
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
