use crate::prelude::*;
use super::helper::{ardrive_post, check_response};

pub const NAME: &str = "ardrive_post_balance";
const DEFINITION: &str = flow_lib::node_definition!("ardrive/ardrive_post_balance.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub token: String,
    pub tx_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = format!("/account/balance/{}", input.token);
    let result = check_response(
        ardrive_post(&ctx, &path)
            .json(&serde_json::json!({ "tx_id": input.tx_id }))
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
