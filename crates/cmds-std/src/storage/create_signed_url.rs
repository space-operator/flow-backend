use super::FileSpec;
use crate::supabase_error;
use flow_lib::command::prelude::*;
use reqwest::{StatusCode, header::AUTHORIZATION};
use rust_decimal::Decimal;
use std::borrow::Cow;

pub const NAME: &str = "storage_create_signed_url";

const DEFINITION: &str = flow_lib::node_definition!("storage/create_signed_url.jsonc");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        Ok(CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .permissions(Permissions { user_tokens: true }))
    });

    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

#[derive(Serialize, Deserialize, Default)]
struct Transform {
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resize: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality: Option<f64>,
}

impl Transform {
    fn all_none(&self) -> bool {
        self.width.is_none()
            && self.height.is_none()
            && self.resize.is_none()
            && self.format.is_none()
            && self.quality.is_none()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum StringOrBool {
    String(String),
    Bool(bool),
}

impl Default for StringOrBool {
    fn default() -> Self {
        StringOrBool::Bool(false)
    }
}

#[derive(Serialize, Deserialize)]
struct Input {
    #[serde(flatten)]
    file: FileSpec,
    #[serde(with = "value::decimal")]
    expires_in: Decimal,
    #[serde(default)]
    transform: Transform,
    #[serde(default)]
    download: StringOrBool,
}

#[derive(Serialize)]
struct Output {
    url: String,
}

#[allow(non_snake_case)]
#[derive(Serialize)]
struct RequestBody {
    #[serde(with = "rust_decimal::serde::float")]
    expiresIn: Decimal,
    #[serde(skip_serializing_if = "Transform::all_none")]
    transform: Transform,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct SuccessBody {
    signedURL: String,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let key = input.file.key(&ctx.flow_owner().id);
    let url = format!(
        "{}/storage/v1/object/sign/{}",
        ctx.endpoints().supabase,
        key
    );
    tracing::debug!("using URL: {}", url);
    let mut req = ctx.http().post(url);

    req = req.header(AUTHORIZATION, ctx.get_jwt_header().await?);

    let body = serde_json::value::to_raw_value(&RequestBody {
        expiresIn: input.expires_in,
        transform: input.transform,
    })?;
    tracing::debug!("using body: {}", body.get());

    let resp = req.json(&body).send().await?;

    match resp.status() {
        StatusCode::OK => {
            let body = resp.json::<SuccessBody>().await?;
            // https://github.com/supabase/storage-js/blob/fa44be8156295ba6320ffeff96bdf91016536a46/src/packages/StorageFileApi.ts#L395-L397
            let download = match input.download {
                StringOrBool::String(s) => Cow::Owned(format!("&download={s}")),
                StringOrBool::Bool(true) => Cow::Borrowed("&download="),
                StringOrBool::Bool(false) => Cow::Borrowed(""),
            };
            Ok(Output {
                url: format!(
                    "{}/storage/v1{}{}",
                    ctx.endpoints().supabase,
                    body.signedURL,
                    download
                ),
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
