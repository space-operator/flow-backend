use crate::supabase_error;
use flow_lib::command::prelude::*;
use reqwest::{StatusCode, header::AUTHORIZATION};
use serde_json::json;
use std::path::PathBuf;

pub const NAME: &str = "storage_list";

const DEFINITION: &str = flow_lib::node_definition!("storage/list.json");

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
    bucket: String,
    #[serde(default)]
    path: PathBuf,
}

#[derive(Serialize)]
struct Output {
    files: Vec<String>,
}

async fn run(mut ctx: CommandContext, input: Input) -> Result<Output, CommandError> {
    let path = input.path;
    let prefix = if ["user-storages", "user-public-storages"].contains(&input.bucket.as_str()) {
        format!("{}/{}", ctx.flow_owner().id, path.display())
    } else {
        format!("{}", path.display())
    };
    let url = format!(
        "{}/storage/v1/object/list/{}",
        ctx.endpoints().supabase,
        input.bucket,
    );
    tracing::debug!("using URL: {}", url);
    tracing::debug!("using prefix: {}", prefix);
    let req = ctx
        .http()
        .post(url)
        .header(AUTHORIZATION, ctx.get_jwt_header().await?);

    let body = json!({
        "prefix": &prefix,
        "limit": 1000,
        "offset": 0,
        "sortBy": {
            "column": "name",
            "order": "asc"
        },
    });

    let resp = req.json(&body).send().await?;

    match resp.status() {
        StatusCode::OK => {
            let body = resp.json::<JsonValue>().await?;
            #[derive(Deserialize)]
            struct File {
                name: PathBuf,
            }
            let files = serde_json::from_value::<Vec<File>>(body)?;
            Ok(Output {
                files: files
                    .into_iter()
                    .map(|f| path.join(f.name).display().to_string())
                    .collect(),
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
