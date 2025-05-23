use super::FileSpec;
use crate::supabase_error;
use flow_lib::command::prelude::*;
use reqwest::{StatusCode, header::AUTHORIZATION};

pub const NAME: &str = "storage_delete";

const DEFINITION: &str = flow_lib::node_definition!("storage/delete.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        Ok(CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .permissions(Permissions { user_tokens: true }))
    });

    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize)]
struct Output {
    key: String,
}

async fn run(mut ctx: CommandContext, input: FileSpec) -> Result<Output, CommandError> {
    let key = input.key(&ctx.flow_owner().id);
    let url = format!("{}/storage/v1/object/{}", ctx.endpoints().supabase, key);
    tracing::debug!("using URL: {}", url);
    let resp = ctx
        .http()
        .delete(url)
        .header(AUTHORIZATION, ctx.get_jwt_header().await?)
        .send()
        .await?;

    match resp.status() {
        StatusCode::OK => Ok(Output { key }),
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
