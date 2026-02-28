use crate::prelude::*;
use super::helper::{ardrive_get, check_response};

pub const NAME: &str = "ardrive_get_price_bytes";
const DEFINITION: &str = flow_lib::node_definition!("ardrive/ardrive_get_price_bytes.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub byte_count: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = format!("/price/bytes/{}", input.byte_count);
    let result = check_response(ardrive_get(&ctx, &path).send().await?).await?;
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
