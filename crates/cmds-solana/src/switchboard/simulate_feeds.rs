use crate::prelude::*;

use super::helper::{check_response, CROSSBAR_URL};

pub const NAME: &str = "switchboard_simulate_feeds";
const DEFINITION: &str =
    flow_lib::node_definition!("switchboard/switchboard_simulate_feeds.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub feed_pubkeys: Vec<String>,
    #[serde(default = "default_cluster")]
    pub cluster: String,
    #[serde(default)]
    pub crossbar_url: Option<String>,
}

fn default_cluster() -> String {
    "Mainnet".to_string()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub results: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let base_url = input
        .crossbar_url
        .as_deref()
        .unwrap_or(CROSSBAR_URL);

    let url = format!("{}/simulate/solana", base_url.trim_end_matches('/'));

    let body = serde_json::json!({
        "cluster": input.cluster,
        "feeds": input.feed_pubkeys,
    });

    let resp = ctx
        .http()
        .post(&url)
        .json(&body)
        .send()
        .await?;

    let results = check_response(resp).await?;

    Ok(Output { results })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_input_parsing() {
        let input = value::map! {
            "feed_pubkeys" => value::Value::Array(vec![
                value::Value::String("7Zi9DkMHHB2MzAHNnR4GiXHe7TWQrJkUMh1CLQWF5qR".into()),
            ]),
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
        let parsed = result.unwrap();
        assert_eq!(parsed.cluster, "Mainnet");
        assert!(parsed.crossbar_url.is_none());
    }
}
