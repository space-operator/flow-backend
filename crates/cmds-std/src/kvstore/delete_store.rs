use crate::supabase_error;
use anyhow::anyhow;
use flow_lib::command::prelude::*;
use reqwest::{StatusCode, header::AUTHORIZATION};

pub const NAME: &str = "kv_delete_store";

const DEFINITION: &str = flow_lib::node_definition!("kvstore/delete_store.json");

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
}

#[derive(Serialize)]
struct Output {}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let mut req = ctx
        .http
        .post(format!("{}/kv/delete_store", ctx.endpoints.flow_server))
        .json(&input);
    req = req.header(AUTHORIZATION, ctx.get_jwt_header().await?);
    let resp = req.send().await.map_err(|e| anyhow!("HTTP error: {}", e))?;
    match resp.status() {
        StatusCode::OK => Ok(Output {}),
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
