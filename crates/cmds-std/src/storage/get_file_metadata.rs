use super::FileSpec;
use crate::supabase_error;
use anyhow::anyhow;
use flow_lib::command::prelude::*;
use reqwest::{
    StatusCode,
    header::{AUTHORIZATION, CONTENT_TYPE, LAST_MODIFIED},
};

pub const NAME: &str = "storage_get_file_metadata";

const DEFINITION: &str = flow_lib::node_definition!("storage/get_file_metadata.json");

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
    content_type: String,
    last_modified: String,
}

async fn run(mut ctx: CommandContextX, input: FileSpec) -> Result<Output, CommandError> {
    let key = input.key(&ctx.flow_owner().id);
    let url = format!(
        "{}/storage/v1/object/info/authenticated/{}",
        ctx.endpoints().supabase, key
    );
    tracing::debug!("using URL: {}", url);
    let resp = ctx
        .http()
        .head(url)
        .header(AUTHORIZATION, ctx.get_jwt_header().await?)
        .send()
        .await?;

    match resp.status() {
        StatusCode::OK => {
            let headers = resp.headers();
            Ok(Output {
                key,
                content_type: String::from_utf8_lossy(
                    headers
                        .get(CONTENT_TYPE)
                        .ok_or_else(|| anyhow!("missing header: content-type"))?
                        .as_bytes(),
                )
                .to_string(),
                last_modified: String::from_utf8_lossy(
                    headers
                        .get(LAST_MODIFIED)
                        .ok_or_else(|| anyhow!("missing header: last-modified"))?
                        .as_bytes(),
                )
                .to_string(),
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
