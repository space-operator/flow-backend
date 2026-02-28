use crate::prelude::*;
use super::helper::{bridge_delete, check_response};

pub const NAME: &str = "bridge_delete_crypto_return_policy";
const DEFINITION: &str = flow_lib::node_definition!("bridge/bridge_delete_crypto_return_policy.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub policy_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = format!("/v0/crypto_return_policies/{}", input.policy_id);
    let result = check_response(
        bridge_delete(&ctx, &path, &input.api_key)
            .send()
            .await?,
    )
    .await?;
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
