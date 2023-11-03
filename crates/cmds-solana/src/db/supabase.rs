use crate::prelude::*;
use anyhow::anyhow;
use flow_lib::config::node::Permissions;
use reqwest::{header::AUTHORIZATION, StatusCode};

// Command Name
const NAME: &str = "supabase";

const DEFINITION: &str = include_str!("../../../../node-definitions/db/supabase.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache = BuilderCache::new(|| {
        Ok(CmdBuilder::new(DEFINITION)?
            .check_name(NAME)?
            .permissions(Permissions { user_tokens: true }))
    });

    Ok(CACHE.clone()?.build(run))
}

inventory::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    pub string: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub res: HashMap<String, String>,
}

async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
    // info!("{:#?}", ctx.environment);

    // info!("{:#?}", ctx.user.id);
    // let bearer = ctx.environment.get("authorization_bearer").unwrap();
    // let apikey = ctx.environment.get("apikey").unwrap();

    // // headers
    // let mut headers = reqwest::header::HeaderMap::new();
    // headers.insert(
    //     "authorization_bearer",
    //     HeaderValue::from_str(&bearer).unwrap(),
    // );
    // headers.insert("apikey", HeaderValue::from_str(&apikey).unwrap());

    let mut req = ctx
        .http
        .post(format!("{}/rest/v1/users_nft", ctx.endpoints.supabase))
        .json(&input.string);

    req = req.header(AUTHORIZATION, ctx.get_jwt_header().await?);

    let resp = req.send().await.map_err(|e| anyhow!("HTTP error: {}", e))?;

    match resp.status() {
        StatusCode::OK => Ok(Output {
            res: resp.json::<HashMap<String, String>>().await?,
        }),
        code => Err(anyhow!("HTTP error: {}", code)),
    }
    // https://hyjboblkjeevkzaqsyxe.supabase.co/rest/v1/users_nft?id=eq.1&select=*
}
