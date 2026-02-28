use crate::prelude::*;
use super::helper::{bridge_post, check_response};

pub const NAME: &str = "bridge_reactivate_virtual_account";
const DEFINITION: &str = flow_lib::node_definition!("bridge/bridge_reactivate_virtual_account.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub customer_id: String,
    pub virtual_account_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = format!("/v0/customers/{}/virtual_accounts/{}/reactivate", input.customer_id, input.virtual_account_id);
    let result = check_response(
        bridge_post(&ctx, &path, &input.api_key)
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
