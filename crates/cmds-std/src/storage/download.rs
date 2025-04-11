use super::FileSpec;
use crate::supabase_error;
use flow_lib::command::prelude::*;
use reqwest::{
    StatusCode,
    header::{AUTHORIZATION, CONTENT_TYPE},
};

pub const NAME: &str = "storage_download";

const DEFINITION: &str = flow_lib::node_definition!("storage/download.json");

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
    content: Value,
    size: u64,
    content_type: String,
}

async fn run(mut ctx: CommandContextX, input: FileSpec) -> Result<Output, CommandError> {
    let key = input.key(&ctx.flow_owner.id);
    let url = format!(
        "{}/storage/v1/object/authenticated/{}",
        ctx.endpoints().supabase, key
    );
    tracing::debug!("using URL: {}", url);
    let resp = ctx
        .http
        .get(url)
        .header(AUTHORIZATION, ctx.get_jwt_header().await?)
        .send()
        .await?;

    match resp.status() {
        StatusCode::OK => {
            let content_type = resp
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|t| t.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_owned();
            let bytes = resp.bytes().await?;
            let size = bytes.len() as u64;
            let content = match std::str::from_utf8(&bytes) {
                Ok(_) => Value::String(unsafe { String::from_utf8_unchecked(bytes.into()) }),
                Err(_) => Value::Bytes(bytes),
            };
            Ok(Output {
                content,
                size,
                content_type,
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
