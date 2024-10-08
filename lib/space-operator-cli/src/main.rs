use chrono::Utc;
use clap::{error, CommandFactory, Parser, Subcommand};
use directories::ProjectDirs;
use error_stack::{Report, ResultExt};
use futures::{io::AllowStdIo, AsyncBufReadExt};
use postgrest::Postgrest;
use reqwest::{
    header::{HeaderName, HeaderValue, AUTHORIZATION},
    StatusCode,
};
use schema::{CommandDefinition, CommandId};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt::Display,
    io::{stdin, BufReader, Write},
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;
use url::Url;
use uuid::Uuid;

pub mod schema;

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

async fn refresh(
    http: &reqwest::Client,
    info: &get_info::Output,
    refresh_token: &str,
    user_id: &Uuid,
) -> Result<claim_token::Output, Report<Error>> {
    #[derive(Serialize)]
    struct Body<'a> {
        refresh_token: &'a str,
    }

    #[derive(Deserialize)]
    struct Resp {
        access_token: String,
        expires_in: u32,
        refresh_token: String,
    }

    let resp = http
        .post(
            info.supabase_url
                .join("/auth/v1/token?grant_type=refresh_token")
                .change_context(Error::Url)?,
        )
        .header(HeaderName::from_static("apikey"), &info.anon_key)
        .json(&Body { refresh_token })
        .send()
        .await
        .change_context(Error::Http)?;

    let resp = read_json_response::<Resp, GoTrueErrorBody>(resp).await?;

    Ok(claim_token::Output {
        user_id: *user_id,
        access_token: resp.access_token,
        refresh_token: resp.refresh_token,
        expires_at: Utc::now() + chrono::Duration::seconds(resp.expires_in as i64),
    })
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
    /// Manage your nodes
    Nodes {
        #[command(subcommand)]
        command: NodesCommands,
    },
}
#[derive(Subcommand, Debug)]
enum NodesCommands {
    /// Upload nodes
    Upload {
        /// Path to JSON node definition file
        path: PathBuf,
        /// Only print diff, don't do anything
        #[arg(long)]
        dry_run: bool,
        /// Don't ask for confirmation
        #[arg(long)]
        no_confirm: bool,
    },
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub flow_server: Url,
    pub info: get_info::Output,
    pub apikey: String,
    pub jwt: claim_token::Output,
}

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("response from server: {}", .0)]
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
    #[error("failed to parse application data")]
    ParseData,
    #[error("failed to write application data {}", .0.display())]
    WriteData(PathBuf),
    #[error("failed to read application data {}", .0.display())]
    ReadData(PathBuf),
    #[error("failed to read file {}", .0.display())]
    ReadFile(PathBuf),
    #[error("failed to parse node definition")]
    ParseNodeDefinition,
    #[error("{}", .0)]
    Unimplemented(&'static str),
    #[error("you are not logged in")]
    NotLogin,
    #[error("JSON error")]
    Json,
    #[error("token refresh error")]
    TokenRefresh,
}

#[derive(Deserialize, ThisError, Debug)]
#[error("{error}")]
pub struct FlowServerErrorBody {
    pub error: String,
}

#[derive(Deserialize, ThisError, Debug)]
#[error("{msg}")]
pub struct GoTrueErrorBody {
    pub msg: String,
}

#[derive(Deserialize, ThisError, Debug)]
#[serde(untagged)]
pub enum PostgrestErrorBody {
    #[error("{error}")]
    Error { error: String },
    #[error("{message}")]
    Postgrest {
        message: String,
        details: Option<String>,
        hint: Option<String>,
    },
}

async fn read_json_response<T: DeserializeOwned, E: Display + DeserializeOwned>(
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
        match serde_json::from_slice::<E>(&bytes) {
            Ok(body) => Err(Error::ErrorResponse(body.to_string()).into()),
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
    read_json_response::<_, FlowServerErrorBody>(resp).await
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
    read_json_response::<_, FlowServerErrorBody>(resp).await
}

async fn read_file<P: AsRef<Path>>(path: P) -> Result<String, Report<Error>> {
    tokio::fs::read_to_string(path.as_ref())
        .await
        .change_context_lazy(|| Error::ReadFile(path.as_ref().to_owned()))
}

pub struct ApiClient {
    http: reqwest::Client,
    pg: postgrest::Postgrest,
    config: Config,
}

impl ApiClient {
    pub fn from_config(config: Config) -> Result<Self, Report<Error>> {
        let http = reqwest::Client::new();
        let pg = Postgrest::new_with_client(
            config
                .info
                .supabase_url
                .join("/rest/v1")
                .change_context(Error::Url)?,
            http.clone(),
        )
        .insert_header(HeaderName::from_static("apikey"), &config.info.anon_key);
        Ok(Self { pg, http, config })
    }

    pub async fn load() -> Result<Self, Report<Error>> {
        let path = Self::data_file_full_path()?;
        let text = tokio::fs::read_to_string(&path)
            .await
            .change_context_lazy(|| Error::ReadData(path.clone()))?;
        let config: Config = toml::from_str(&text).change_context(Error::ParseData)?;
        Self::from_config(config)
    }

    pub async fn new(flow_server: Url, apikey: String) -> Result<Self, Report<Error>> {
        let http = reqwest::Client::new();
        let token = claim_token(&http, &flow_server, &apikey).await?;
        let info = get_info(&http, &flow_server, &token.access_token).await?;
        let config = Config {
            flow_server,
            info,
            apikey,
            jwt: token,
        };
        Self::from_config(config)
    }

    async fn get_access_token(&mut self) -> Result<String, Report<Error>> {
        let now = chrono::Utc::now();
        if now >= self.config.jwt.expires_at + chrono::Duration::minutes(1) {
            self.config.jwt = refresh(
                &self.http,
                &self.config.info,
                &self.config.jwt.refresh_token,
                &self.config.jwt.user_id,
            )
            .await
            .change_context(Error::TokenRefresh)?;
        }
        Ok(self.config.jwt.access_token.clone())
    }

    pub async fn insert_node(
        &mut self,
        def: &CommandDefinition,
    ) -> Result<CommandId, Report<Error>> {
        #[derive(Serialize)]
        struct InsertNode<'a> {
            #[serde(flatten)]
            def: &'a CommandDefinition,
            #[serde(rename = "isPublic")]
            is_public: bool,
            unique_node_id: String,
            name: &'a str,
        }

        let body = serde_json::to_string(&InsertNode {
            def,
            is_public: true,
            unique_node_id: format!(
                "{}.{}.{}",
                self.config.jwt.user_id, def.data.node_id, def.data.version
            ),
            name: &def.data.node_id,
        })
        .change_context(Error::Json)?;

        let resp = self
            .pg
            .from("nodes")
            .auth(self.get_access_token().await?)
            .insert(body)
            .select("id")
            .single()
            .execute()
            .await
            .change_context(Error::Postgrest)?;

        #[derive(Deserialize)]
        struct Resp {
            id: CommandId,
        }

        let resp = read_json_response::<Resp, PostgrestErrorBody>(resp).await?;

        Ok(resp.id)
    }

    pub async fn get_my_native_node(
        &mut self,
        node_id: &str,
    ) -> Result<Option<(CommandId, CommandDefinition)>, Report<Error>> {
        #[derive(Serialize)]
        struct Query<'a> {
            node_id: &'a str,
        }
        let resp = self
            .pg
            .from("nodes")
            .auth(self.get_access_token().await?)
            .eq("user_id", self.config.jwt.user_id.to_string())
            .eq("type", "native")
            .cs(
                "data",
                serde_json::to_string(&Query { node_id }).change_context(Error::Json)?,
            )
            .select("*")
            .execute()
            .await
            .change_context(Error::Postgrest)?;
        let mut nodes =
            read_json_response::<Vec<serde_json::Value>, PostgrestErrorBody>(resp).await?;
        error_stack::ensure!(
            nodes.len() <= 1,
            Error::ErrorResponse("more than 1 native nodes".to_owned())
        );

        match nodes.pop() {
            Some(json) => {
                #[derive(Deserialize)]
                struct Row {
                    id: CommandId,
                    #[serde(flatten)]
                    def: CommandDefinition,
                }

                let row = serde_json::from_value::<Row>(json).change_context(Error::Json)?;
                Ok(Some((row.id, row.def)))
            }
            None => Ok(None),
        }
    }

    pub fn data_dir() -> Result<PathBuf, Report<Error>> {
        Ok(project_dirs()?.data_dir().to_owned())
    }

    pub const fn data_file_name() -> &'static str {
        "data.toml"
    }

    pub fn data_file_full_path() -> Result<PathBuf, Report<Error>> {
        Ok(Self::data_dir()?.join(Self::data_file_name()).to_owned())
    }

    pub async fn save_application_data(&self) -> Result<(), Report<Error>> {
        let base = Self::data_dir()?;

        tokio::fs::create_dir_all(&base)
            .await
            .change_context_lazy(|| Error::WriteData(base.clone()))?;

        let path = base.join(Self::data_file_name());

        let data = toml::to_string_pretty(&self.config).change_context(Error::SerializeData)?;
        tokio::fs::write(&path, data)
            .await
            .change_context_lazy(|| Error::WriteData(path.clone()))?;
        Ok(())
    }

    pub async fn get_username(&mut self) -> Result<Option<String>, Report<Error>> {
        let resp = self
            .pg
            .from("users_public")
            .auth(self.get_access_token().await?)
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

        read_json_response::<Body, PostgrestErrorBody>(resp)
            .await
            .map(|body| body.username)
    }
}

fn project_dirs() -> Result<ProjectDirs, Report<Error>> {
    Ok(ProjectDirs::from("com", "spaceoperator", "spo").ok_or(Error::Dir)?)
}

async fn ask() -> bool {
    print!("Upload? (y/n) ");
    std::io::stdout().flush().ok();

    let mut stdin = AllowStdIo::new(BufReader::new(stdin()));
    let mut answer = String::new();
    stdin.read_line(&mut answer).await.ok();

    answer.trim().to_lowercase() == "y"
}

async fn upload_node(path: &PathBuf, dry_run: bool, no_confirm: bool) -> Result<(), Report<Error>> {
    let mut client = ApiClient::load().await.change_context(Error::NotLogin)?;
    let text = read_file(path).await?;
    let def = serde_json::from_str::<schema::CommandDefinition>(&text)
        .change_context(Error::ParseNodeDefinition)?;
    if def.r#type != "native" {
        return Err(
            Error::Unimplemented("we only support uploading native nodes at the moment").into(),
        );
    }
    match client.get_my_native_node(&def.data.node_id).await? {
        Some((id, db)) => {
            dbg!(id);
        }
        None => {
            println!("Command is not in database");
            if dry_run {
                return Ok(());
            }
            if !no_confirm {
                let yes = ask().await;
                if !yes {
                    return Ok(());
                }
            }
            let id = client.insert_node(&def).await?;
            println!("Inserted new node {}", id);
        }
    }

    Ok(())
}

async fn run() -> Result<(), Report<Error>> {
    let args = Args::parse();
    let flow_server = args
        .url
        .unwrap_or_else(|| Url::parse("https://dev-api.spaceoperator.com").unwrap());
    match &args.command {
        Some(Commands::Login {}) => {
            println!("Go to https://spaceoperator.com/dashboard/profile/apikey go generate a key");
            println!("Please paste your API key below");
            let mut key = String::new();
            let mut stdin = AllowStdIo::new(BufReader::new(stdin()));
            stdin.read_line(&mut key).await.ok();
            let key = key.trim().to_owned();

            let mut client = ApiClient::new(flow_server, key).await?;
            let username = client.get_username().await?.unwrap_or_default();
            println!("Logged in as {:?}", username);
            client.save_application_data().await?;
        }
        Some(Commands::Nodes { command }) => match command {
            NodesCommands::Upload {
                path,
                dry_run,
                no_confirm,
            } => {
                upload_node(path, *dry_run, *no_confirm).await?;
            }
        },
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
        eprintln!("Error: {:#?}", error);
    }
}
