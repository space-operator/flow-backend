use crate::command::{prelude::*, supabase_error};
use anyhow::anyhow;
use flow_lib::config::node::Permissions;
use reqwest::{header::AUTHORIZATION, StatusCode};

pub const NAME: &str = "kv_write_item";

const DEFINITION: &str = include_str!("write_item.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        Ok(CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .permissions(Permissions { user_tokens: true }))
    });

    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize)]
struct Input {
    store: String,
    key: String,
    value: Value,
}

#[derive(Serialize)]
struct Output {
    #[serde(skip_serializing_if = "Option::is_none")]
    old_value: Option<Value>,
}

#[derive(Deserialize)]
struct SuccessBody {
    old_value: Option<Value>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let mut req = ctx
        .http
        .post(format!("{}/kv/write_item", ctx.endpoints.flow_server))
        .json(&input);
    req = req.header(AUTHORIZATION, ctx.get_jwt_header().await?);
    let resp = req.send().await.map_err(|e| anyhow!("HTTP error: {}", e))?;
    match resp.status() {
        StatusCode::OK => {
            let body = resp.json::<SuccessBody>().await?;
            Ok(Output {
                old_value: body.old_value,
            })
        }
        code => Err(supabase_error(code, resp).await),
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
