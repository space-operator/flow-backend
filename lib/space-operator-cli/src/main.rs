use clap::{CommandFactory, Parser, Subcommand};
use directories::ProjectDirs;
use error_stack::{Report, ResultExt};
use futures::{io::AllowStdIo, AsyncBufReadExt};
use postgrest::Postgrest;
use reqwest::{
    header::{HeaderName, HeaderValue, AUTHORIZATION},
    StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io::{stdin, BufReader};
use thiserror::Error as ThisError;
use url::Url;

pub mod claim_token {
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    use super::*;

    #[derive(Deserialize, Serialize, Debug)]
    pub struct Output {
        pub user_id: Uuid,
        pub access_token: String,
        pub refresh_token: String,
        #[serde(with = "chrono::serde::ts_seconds")]
        pub expires_at: DateTime<Utc>,
    }
}

pub mod get_info {
    use url::Url;

    use super::*;

    #[derive(Deserialize, Serialize, Debug)]
    pub struct Output {
        pub supabase_url: Url,
        pub anon_key: String,
    }
}

#[derive(Parser, Debug)]
#[command(name = "spo")]
struct Args {
    /// URL of flow-server to use (default: https://dev-api.spaceoperator.com)
    #[arg(long)]
    url: Option<Url>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Login to Space Operator using API key
    Login {},
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub flow_server: Url,
    pub info: get_info::Output,
    pub apikey: String,
    pub jwt: claim_token::Output,
}

pub struct ApiClient {
    http: reqwest::Client,
    pg: postgrest::Postgrest,
    config: Config,
}

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("{}", .0)]
    ErrorResponse(String),
    #[error("{}: {}", .0, .1)]
    UnknownResponse(StatusCode, String),
    #[error("invalid API key")]
    InvalidApiKey,
    #[error("HTTP error")]
    Http,
    #[error("URL error")]
    Url,
    #[error("DB error")]
    Postgrest,
    #[error("could not find location to save application data")]
    Dir,
    #[error("failed to serialize application data")]
    SerializeData,
    #[error("failed to write application data")]
    WriteData,
}

#[derive(Deserialize)]
pub struct ErrorBody {
    pub error: String,
}

async fn read_json_response<T: DeserializeOwned>(
    resp: reqwest::Response,
) -> Result<T, Report<Error>> {
    let code = resp.status();
    let bytes = resp.bytes().await.change_context(Error::Http)?;
    if code.is_success() {
        match serde_json::from_slice::<T>(&bytes) {
            Ok(body) => Ok(body),
            Err(error) => {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                Err(Report::new(error).change_context(Error::UnknownResponse(code, text).into()))
            }
        }
    } else {
        match serde_json::from_slice::<ErrorBody>(&bytes) {
            Ok(body) => Err(Error::ErrorResponse(body.error).into()),
            Err(_) => {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                Err(Error::UnknownResponse(code, text).into())
            }
        }
    }
}

async fn claim_token(
    http: &reqwest::Client,
    flow_server: &Url,
    apikey: &str,
) -> Result<claim_token::Output, Report<Error>> {
    let apikey = HeaderValue::from_str(apikey).change_context(Error::InvalidApiKey)?;
    let resp = http
        .post(
            flow_server
                .join("/auth/claim_token")
                .change_context(Error::Url)?,
        )
        .header(HeaderName::from_static("x-api-key"), apikey)
        .send()
        .await
        .change_context(Error::Http)?;
    read_json_response(resp).await
}

async fn get_info(
    http: &reqwest::Client,
    flow_server: &Url,
    access_token: &str,
) -> Result<get_info::Output, Report<Error>> {
    let resp = http
        .get(flow_server.join("/info").change_context(Error::Url)?)
        .header(AUTHORIZATION, format!("Bearer {}", access_token))
        .send()
        .await
        .change_context(Error::Http)?;
    read_json_response(resp).await
}

impl ApiClient {
    pub async fn new(flow_server: Url, apikey: String) -> Result<Self, Report<Error>> {
        let http = reqwest::Client::new();
        let token = claim_token(&http, &flow_server, &apikey).await?;
        let info = get_info(&http, &flow_server, &token.access_token).await?;
        let pg = Postgrest::new_with_client(
            info.supabase_url
                .join("/rest/v1")
                .change_context(Error::Url)?,
            http.clone(),
        )
        .insert_header(HeaderName::from_static("apikey"), &info.anon_key);
        Ok(Self {
            pg,
            http,
            config: Config {
                flow_server,
                info,
                apikey,
                jwt: token,
            },
        })
    }

    pub async fn save_application_data(&self) -> Result<(), Report<Error>> {
        let dirs = project_dirs()?;
        let data = toml::to_string_pretty(&self.config).change_context(Error::SerializeData)?;
        let base = dirs.data_dir();
        tokio::fs::create_dir_all(base)
            .await
            .change_context(Error::WriteData)
            .attach_printable_lazy(|| format!("failed to create directories {}", base.display()))?;
        let path = base.join("data.toml");
        tokio::fs::write(&path, data)
            .await
            .change_context(Error::WriteData)
            .attach_printable_lazy(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub async fn get_username(&self) -> Result<Option<String>, Report<Error>> {
        let resp = self
            .pg
            .from("users_public")
            .auth(&self.config.jwt.access_token)
            .eq("user_id", self.config.jwt.user_id.to_string())
            .select("username")
            .single()
            .execute()
            .await
            .change_context(Error::Postgrest)?;

        #[derive(Deserialize)]
        struct Body {
            username: Option<String>,
        }

        read_json_response::<Body>(resp)
            .await
            .map(|body| body.username)
    }
}

fn project_dirs() -> Result<ProjectDirs, Report<Error>> {
    Ok(ProjectDirs::from("com", "spaceoperator", "spo").ok_or(Error::Dir)?)
}

async fn run() -> Result<(), Report<Error>> {
    let args = Args::parse();
    let flow_server = args
        .url
        .unwrap_or_else(|| Url::parse("https://dev-api.spaceoperator.com").unwrap());
    let mut stdin = AllowStdIo::new(BufReader::new(stdin()));
    match &args.command {
        Some(Commands::Login {}) => {
            println!("Go to https://spaceoperator.com/dashboard/profile/apikey go generate a key");
            println!("Please paste your API key below");
            let mut key = String::new();
            stdin.read_line(&mut key).await.ok();
            let key = key.trim().to_owned();

            let client = ApiClient::new(flow_server, key).await?;
            let username = client.get_username().await?.unwrap_or_default();
            println!("Logged in as {:?}", username);
            client.save_application_data().await?;
        }
        None => {
            Args::command().print_help().ok();
        }
    }
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    Report::set_color_mode(error_stack::fmt::ColorMode::Color);
    Report::install_debug_hook::<std::panic::Location>(|_, _| {});
    if let Err(error) = run().await {
        eprintln!("Error: {:?}", error);
    }
}
