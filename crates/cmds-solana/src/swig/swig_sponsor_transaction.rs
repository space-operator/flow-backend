use crate::prelude::*;
use super::{paymaster_post, check_response};

const NAME: &str = "swig_sponsor_transaction";
const DEFINITION: &str = flow_lib::node_definition!("swig/swig_sponsor_transaction.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub base58_encoded_transaction: String,
    #[serde(default = "default_network")]
    pub network: Option<String>,
}

fn default_network() -> Option<String> { Some("mainnet".to_string()) }

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut body = serde_json::json!({
        "base58_encoded_transaction": input.base58_encoded_transaction,
    });
    if let Some(network) = &input.network {
        body["network"] = serde_json::json!(network);
    }

    let result = check_response(
        paymaster_post(&ctx, "/sponsor", &input.api_key)
            .json(&body)
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
