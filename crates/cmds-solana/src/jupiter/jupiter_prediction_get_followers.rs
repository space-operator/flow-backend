//! Jupiter Prediction Get Followers - Get followers list
//!
//! Jupiter API: GET /prediction/v1/followers/{address}

use crate::prelude::*;

pub const NAME: &str = "jupiter_prediction_get_followers";
const DEFINITION: &str = flow_lib::node_definition!("jupiter/jupiter_prediction_get_followers.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = format!("https://api.jup.ag/prediction/v1/followers/{}", input.address);

    let req = ctx
        .http()
        .get(&url)
        .header("x-api-key", &input.api_key);

    let resp = req.send().await?;

    if !resp.status().is_success() {
        return Err(CommandError::msg(format!(
            "Jupiter API error: {} {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        )));
    }

    let response: JsonValue = resp.json().await?;

    Ok(Output { result: response })
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
            "api_key" => "test-api-key",
            "address" => "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
