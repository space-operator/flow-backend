use chrono::{DateTime, Utc};
use flow_lib::command::prelude::*;
use reqwest::{StatusCode, header::CONTENT_TYPE};
use serde_json::json;

pub const NAME: &str = "das_api";

const DEFINITION: &str = flow_lib::node_definition!("das_api.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));

    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize)]
enum DasMethod {
    GetAsset,
    GetAssetProof,
    GetAssetsByOwner,
    GetAssetsByCreator,
    GetAssetsByAuthority,
    GetAssetsbyCreator,
    GetAssetsByGroup,
    SearchAssets,
    GetSignaturesForAsset,
    GetTokenAccounts,
}

#[derive(Serialize, Deserialize)]
struct Input {
    url: String,
    params: JsonValue,
    method: DasMethod,
    id: Option<String>,
}

#[derive(Serialize)]
struct Output {
    response: JsonValue,
}

async fn run(ctx: Context, input: Input) -> Result<Output, CommandError> {
    let content_type = "application/json";

    // get time
    let now: DateTime<Utc> = Utc::now();
    let formatted_now = now.format("%m-%d-%y - %r").to_string();

    let body = json!(
        {
            "jsonrpc": "2.0",
            "method": match input.method {
                DasMethod::GetAsset => "getAsset",
                DasMethod::GetAssetProof => "getAssetProof",
                DasMethod::GetAssetsByOwner => "getAssetsByOwner",
                DasMethod::GetAssetsByCreator => "getAssetsByCreator",
                DasMethod::GetAssetsByAuthority => "getAssetsByAuthority",
                DasMethod::GetAssetsbyCreator => "getAssetsbyCreator",
                DasMethod::GetAssetsByGroup => "getAssetsByGroup",
                DasMethod::SearchAssets => "searchAssets",
                DasMethod::GetSignaturesForAsset => "getSignaturesForAsset",
                DasMethod::GetTokenAccounts => "getTokenAccounts",
            },
            "params": input.params,
            "id": input.id.unwrap_or(formatted_now),
        }
    );

    let req = ctx
        .http
        .post(input.url)
        .header(CONTENT_TYPE, content_type)
        .json(&body);

    let resp = req.send().await?;

    match resp.status() {
        StatusCode::OK => {
            let response = resp.json().await?;
            Ok(Output { response })
        }
        code => Err(CommandError::msg(format!("{} {:?}", code, resp))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }
}
