use crate::supabase_error;
use bytes::Bytes;
use flow_lib::command::prelude::*;
use mime_guess::MimeGuess;
use reqwest::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    StatusCode,
};
use std::{borrow::Cow, path::PathBuf};

pub const NAME: &str = "storage_upload";

const DEFINITION: &str = flow_lib::node_definition!("storage/upload.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        Ok(CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .permissions(Permissions { user_tokens: true }))
    });

    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

const fn bool_false() -> bool {
    false
}

const fn default_bucket() -> Cow<'static, str> {
    Cow::Borrowed("user-storages")
}

#[derive(Serialize, Deserialize)]
struct Input {
    #[serde(default = "default_bucket")]
    bucket: Cow<'static, str>,
    path: PathBuf,
    content_type: Option<String>,
    content: Bytes,
    #[serde(default = "bool_false")]
    overwrite: bool,
}

#[derive(Serialize)]
struct Output {
    key: String,
    content_type: String,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct SuccessBody {
    Key: String,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    let auth = ctx.get_jwt_header().await?;

    let content_type = input.content_type.unwrap_or_else(|| {
        MimeGuess::from_path(&input.path)
            .first_raw()
            .unwrap_or_else(|| match std::str::from_utf8(&input.content) {
                Ok(_) => "text/plain",
                Err(_) => "application/octet-stream",
            })
            .to_owned()
    });
    let url = {
        use std::fmt::Write;
        let mut url = format!(
            "{}/storage/v1/object/{}",
            ctx.endpoints.supabase, input.bucket
        );
        if ["user-storages", "user-public-storages"].contains(&input.bucket.as_ref()) {
            write!(&mut url, "/{}", ctx.user.id).unwrap();
        }
        write!(&mut url, "/{}", input.path.display()).unwrap();
        url
    };

    let mut req = ctx
        .http
        .post(url)
        .header(AUTHORIZATION, auth)
        .header(CONTENT_TYPE, &content_type)
        .body(input.content);

    if input.overwrite {
        req = req.header("x-upsert", "true");
    }

    let resp = req.send().await?;

    match resp.status() {
        StatusCode::OK => {
            let resp = resp.json::<SuccessBody>().await?;
            Ok(Output {
                key: resp.Key,
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
