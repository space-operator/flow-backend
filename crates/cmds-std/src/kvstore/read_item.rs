use crate::supabase_error;
use anyhow::anyhow;
use flow_lib::command::prelude::*;
use reqwest::{StatusCode, header::AUTHORIZATION};

pub const NAME: &str = "kv_read_item";

const DEFINITION: &str = flow_lib::node_definition!("kvstore/read_item.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        Ok(CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .permissions(Permissions { user_tokens: true }))
    });

    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize)]
struct Input {
    store: String,
    key: String,
    #[serde(skip_serializing)]
    default: Option<Value>,
}

#[derive(Serialize)]
struct Output {
    value: Value,
    found: bool,
}

#[derive(Deserialize)]
pub struct SuccessBody {
    pub value: Value,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let mut req = ctx
        .http()
        .post(format!("{}/kv/read_item", ctx.endpoints().flow_server))
        .json(&input);
    req = req.header(AUTHORIZATION, ctx.get_jwt_header().await?);
    let resp = req.send().await.map_err(|e| anyhow!("HTTP error: {}", e))?;
    match resp.status() {
        StatusCode::OK => {
            let body = resp.json::<SuccessBody>().await?;
            Ok(Output {
                value: body.value,
                found: true,
            })
        }
        StatusCode::NOT_FOUND => match input.default {
            Some(default) => Ok(Output {
                value: default,
                found: false,
            }),
            None => Err(CommandError::msg("not found")),
        },
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
