//! Jupiter Send Craft Clawback - Craft a clawback transaction
//!
//! Jupiter API: POST /send/v1/craft-clawback

use crate::prelude::*;
use serde_json::json;

pub const NAME: &str = "jupiter_send_craft_clawback";
const DEFINITION: &str = flow_lib::node_definition!("jupiter/jupiter_send_craft_clawback.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub api_key: String,
    #[serde(rename = "invitePDA")]
    pub invite_pda: String,
    pub sender: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub result: JsonValue,
}

async fn run(ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let url = "https://api.jup.ag/send/v1/craft-clawback".to_string();

    let body = json!({
        "invitePDA": input.invite_pda,
        "sender": input.sender,
    });

    let req = ctx
        .http()
        .post(&url)
        .header("x-api-key", &input.api_key)
        .header("Content-Type", "application/json")
        .json(&body);

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
            "invitePDA" => "test-value",
            "sender" => "test-value",
        };
        let result = value::from_map::<Input>(input);
        assert!(result.is_ok(), "Failed to parse input: {:?}", result.err());
    }
}
