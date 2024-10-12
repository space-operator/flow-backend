#![allow(clippy::print_stdout, clippy::print_stderr)]

use cargo_metadata::{Metadata, Package};
use chrono::Utc;
use clap::{ColorChoice, CommandFactory, Parser, Subcommand, ValueEnum};
use directories::ProjectDirs;
use error_stack::{Report, ResultExt};
use futures::{io::AllowStdIo, AsyncBufReadExt};
use postgrest::Postgrest;
use regex::Regex;
use reqwest::{
    header::{HeaderName, HeaderValue, AUTHORIZATION},
    StatusCode,
};
use schema::{CommandDefinition, CommandId};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    borrow::Cow,
    fmt::Display,
    io::{BufReader, Stdin, Write},
    path::{Path, PathBuf},
};
use strum::IntoEnumIterator;
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

fn get_color() -> ColorChoice {
    std::env::var("COLOR")
        .ok()
        .and_then(|var| ColorChoice::from_str(&var, true).ok())
        .unwrap_or_default()
}

#[derive(Parser, Debug)]
#[command(name = "spo")]
#[command(color = get_color())]
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
    /// Manage your nodes (alias: n)
    #[command(alias = "n")]
    Node {
        #[command(subcommand)]
        command: NodeCommands,
    },
}
#[derive(Subcommand, Debug)]
enum NodeCommands {
    /// Generate a new node
    New {
        /// Allow dirty git repository
        #[arg(long)]
        allow_dirty: bool,
        /// Specify which Rust package to add the new node to
        #[arg(long, short)]
        package: Option<String>,
    },
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
    #[error("git error: {}", .0)]
    Gix(&'static str),
    #[error("IO error: {}", .0)]
    Io(&'static str),
    #[error("cargo metadata error")]
    Metadata,
    #[error("package not found: {}", .0)]
    PackageNotFound(String),
    #[error("package is not a library: {}", .0)]
    NotLib(String),
    #[error("invalid value")]
    InvalidValue,
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
                Err(Report::new(error).change_context(Error::UnknownResponse(code, text)))
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

    pub async fn update_node(
        &mut self,
        id: CommandId,
        def: &CommandDefinition,
    ) -> Result<CommandId, Report<Error>> {
        #[derive(Serialize)]
        struct UpdateNode<'a> {
            #[serde(flatten)]
            def: &'a CommandDefinition,
            unique_node_id: String,
            name: &'a str,
        }

        let body = serde_json::to_string_pretty(&UpdateNode {
            def,
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
            .eq("id", id.to_string())
            .update(body)
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

async fn ask(q: &str) -> bool {
    print!("{} (y/n): ", q);
    std::io::stdout().flush().ok();

    let mut stdin = AllowStdIo::new(BufReader::new(stdin()));
    let mut answer = String::new();
    stdin.read_line(&mut answer).await.ok();

    answer.trim().to_lowercase() == "y"
}

struct Line(Option<usize>);

impl std::fmt::Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.0 {
            None => write!(f, "    "),
            Some(idx) => write!(f, "{:<4}", idx + 1),
        }
    }
}

fn print_diff<T: Serialize>(local: &T, db: &T) -> bool {
    use console::{style, Style};
    use similar::{ChangeTag, TextDiff};

    let local_json = serde_json::to_string_pretty(local).unwrap();
    let db_json = serde_json::to_string_pretty(db).unwrap();

    let diff = TextDiff::from_lines(db_json.as_str(), local_json.as_str());

    if diff
        .iter_all_changes()
        .filter(|c| c.tag() != ChangeTag::Equal)
        .count()
        == 0
    {
        println!("No differences");
        return false;
    }

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            println!("{:-^1$}", "-", 80);
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                let (sign, s) = match change.tag() {
                    ChangeTag::Delete => ("-", Style::new().red()),
                    ChangeTag::Insert => ("+", Style::new().green()),
                    ChangeTag::Equal => (" ", Style::new().dim()),
                };
                print!(
                    "{}{} |{}",
                    style(Line(change.old_index())).dim(),
                    style(Line(change.new_index())).dim(),
                    s.apply_to(sign).bold(),
                );
                for (emphasized, value) in change.iter_strings_lossy() {
                    if emphasized {
                        print!("{}", s.apply_to(value).underlined().on_black());
                    } else {
                        print!("{}", s.apply_to(value));
                    }
                }
                if change.missing_newline() {
                    println!();
                }
            }
        }
    }

    true
}

fn is_dirty() -> Result<bool, Report<Error>> {
    let repo =
        gix::ThreadSafeRepository::discover(".").change_context(Error::Gix("open repository"))?;
    Ok(repo
        .to_thread_local()
        .is_dirty()
        .change_context(Error::Gix("get status"))?)
}

fn cargo_metadata() -> Result<Metadata, Report<Error>> {
    cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .change_context(Error::Metadata)
}

fn find_target_crate_by_name<'a>(
    meta: &'a Metadata,
    name: &str,
) -> Result<&'a Package, Report<Error>> {
    let members = meta.workspace_packages();
    Ok(members
        .into_iter()
        .find(|p| p.name == name)
        .ok_or_else(|| Error::PackageNotFound(name.to_owned()))?)
}

fn find_target_crate<'a>(meta: &'a Metadata) -> Result<Option<&'a Package>, Report<Error>> {
    let members = meta.workspace_packages();
    let pwd = std::env::current_dir().change_context(Error::Io("get current dir"))?;
    let member = members.into_iter().find(|p| {
        p.targets.iter().any(|t| t.is_lib())
            && p.manifest_path
                .parent()
                .map(|root| pwd.starts_with(root))
                .unwrap_or(false)
    });
    Ok(member)
}

async fn new_node(allow_dirty: bool, package: &Option<String>) -> Result<(), Report<Error>> {
    if is_dirty()
        .inspect_err(|error| {
            eprintln!("error: {:?}", error);
        })
        .unwrap_or(false)
    {
        if !allow_dirty {
            eprintln!("dirty git repository");
            eprintln!("use --allow-dirty to continue");
            return Ok(());
        }
    }
    let meta = cargo_metadata()?;
    let member = if let Some(name) = package.as_deref() {
        find_target_crate_by_name(&meta, name)?
    } else if let Some(member) = find_target_crate(&meta)? {
        member
    } else {
        eprintln!("could not determine which package to update");
        eprintln!("use `-p` option to specify a package");
        let list = meta
            .workspace_packages()
            .iter()
            .filter(|p| p.targets.iter().any(|t| t.is_lib()))
            .map(|p| format!("\n    {}", p.name))
            .collect::<String>();
        eprintln!("available packages: {}", list);
        return Ok(());
    };
    if !member.targets.iter().any(|p| p.is_lib()) {
        return Err(Report::new(Error::NotLib(member.name.clone())));
    }
    println!("using package: {}", member.name);

    let mut stdin = stdin();

    let identifier_regex = Regex::new(r#"^[[:alpha:]][[:word:]]*$"#).unwrap();
    let identifier_hint =
        "value can only contains characters [a-zA-Z0-9_] and must start with [a-zA-Z]";

    let node_id = Prompt::builder()
        .question("node id: ")
        .check_regex(&identifier_regex)
        .regex_hint(identifier_hint)
        .build()
        .prompt(&mut stdin)
        .await?;

    let display_name = Prompt::builder()
        .question("display name: ")
        .check_regex(&Regex::new(r#"\S+"#).unwrap())
        .regex_hint("value cannot be empty")
        .build()
        .prompt(&mut stdin)
        .await?;

    let description = Prompt::builder()
        .question("description: ")
        .build()
        .prompt(&mut stdin)
        .await?;

    let name_regex = Regex::new(r#"^[[:alpha:]][[:word:]]*$"#).unwrap();
    let name_hint = "value can only contains characters [a-zA-Z0-9_] and must start with [a-zA-Z]";

    let mut inputs = Vec::<schema::Target>::new();
    loop {
        println!("adding node inputs (enter empty name to finish)");

        let name = Prompt::builder()
            .question("name: ")
            .check_regex(&name_regex)
            .allow_empty(true)
            .regex_hint(name_hint)
            .build()
            .prompt(&mut stdin)
            .await?;
        if name.is_empty() {
            break;
        }

        let types = schema::ValueType::iter()
            .map(|t| t.into())
            .collect::<Vec<&'static str>>();
        let type_bound_str = Prompt::builder()
            .question("input type: ")
            .check_list(&types)
            .build()
            .prompt(&mut stdin)
            .await?;
        let _: schema::ValueType = type_bound_str.parse().unwrap();

        let optional = Prompt::builder()
            .question("optional (true/false): ")
            .check_list(&["true", "false"])
            .build()
            .prompt(&mut stdin)
            .await?;
        let optional: bool = optional.parse().unwrap();

        let passthrough = Prompt::builder()
            .question("passthrough (true/false): ")
            .check_list(&["true", "false"])
            .build()
            .prompt(&mut stdin)
            .await?;
        let passthrough: bool = passthrough.parse().unwrap();

        inputs.push(schema::Target {
            name,
            type_bounds: [type_bound_str].into(),
            required: !optional,
            default_value: serde_json::Value::Null,
            passthrough,
            tooltip: String::new(),
        });
    }

    let mut outputs = Vec::<schema::Source>::new();
    loop {
        println!("adding node outputs (enter empty name to finish)");

        let name = Prompt::builder()
            .question("name: ")
            .check_regex(&name_regex)
            .allow_empty(true)
            .regex_hint(name_hint)
            .build()
            .prompt(&mut stdin)
            .await?;
        if name.is_empty() {
            break;
        }

        let types = schema::ValueType::iter()
            .map(|t| t.into())
            .collect::<Vec<&'static str>>();
        let type_bound_str = Prompt::builder()
            .question("output type: ")
            .check_list(&types)
            .build()
            .prompt(&mut stdin)
            .await?;
        let _: schema::ValueType = type_bound_str.parse().unwrap();

        let optional = Prompt::builder()
            .question("optional (true/false): ")
            .check_list(&["true", "false"])
            .build()
            .prompt(&mut stdin)
            .await?;
        let optional: bool = optional.parse().unwrap();

        outputs.push(schema::Source {
            name,
            r#type: type_bound_str,
            tooltip: String::new(),
            optional,
            default_value: serde_json::Value::Null,
        });
    }

    let ins = ask("will this node emit Solana instructions?").await;
    if ins {
        if !outputs.iter().any(|o| o.name == "signature") {
            println!("adding `signature` output");
            outputs.push(schema::Source {
                name: "signature".to_owned(),
                r#type: "signature".to_owned(),
                default_value: serde_json::Value::Null,
                tooltip: String::new(),
                optional: true,
            });
        }
        if !outputs.iter().any(|o| o.name == "submit") {
            println!("adding `submit` output");
            outputs.push(schema::Source {
                name: "submit".to_owned(),
                r#type: "bool".to_owned(),
                default_value: serde_json::Value::Bool(true),
                tooltip: String::new(),
                optional: true,
            });
        }
    }

    dbg!(node_id);
    dbg!(display_name);
    dbg!(description);
    dbg!(inputs);
    dbg!(outputs);
    Ok(())
}

#[derive(bon::Builder)]
struct Prompt<'a> {
    #[builder(into)]
    question: Cow<'a, str>,
    #[builder(default)]
    allow_empty: bool,
    check_regex: Option<&'a Regex>,
    #[builder(into)]
    regex_hint: Option<Cow<'a, str>>,
    check_list: Option<&'a [&'a str]>,
}

impl<'a> Prompt<'a> {
    pub async fn prompt<S: futures::AsyncBufRead + Unpin>(
        &self,
        stdin: &mut S,
    ) -> Result<String, Report<Error>> {
        let mut tries = 5;
        loop {
            match self.prompt_inner(stdin).await {
                Ok(result) => break Ok(result),
                Err(error) => {
                    tries -= 1;
                    if tries == 0 {
                        break Err(error);
                    } else {
                        eprintln!("error: {:?}", error);
                    }
                }
            }
        }
    }

    async fn prompt_inner<S: futures::AsyncBufRead + Unpin>(
        &self,
        stdin: &mut S,
    ) -> Result<String, Report<Error>> {
        let result = {
            loop {
                if self.check_list.is_some() {
                    print!("(?) ");
                }
                print!("{}", self.question);
                std::io::stdout().flush().ok();
                let mut result = String::new();
                stdin.read_line(&mut result).await.ok();
                let result = result.trim();
                if let Some(list) = self.check_list {
                    if result == "?" {
                        let availables = format!("possible values: {}", list.join(", "));
                        println!("{}", availables);
                        continue;
                    }
                }
                break result.to_owned();
            }
        };

        if self.allow_empty && result.is_empty() {
            return Ok(result.to_owned());
        }
        if let Some(re) = &self.check_regex {
            if !re.is_match(&result) {
                let mut report = Report::new(Error::InvalidValue);
                if let Some(hint) = &self.regex_hint {
                    report = report.attach_printable(hint.clone().into_owned());
                }
                return Err(report);
            }
        }
        if let Some(list) = self.check_list {
            if !list.contains(&result.as_str()) {
                let availables = format!("possible values: {}", list.join(", "));
                let report = Report::new(Error::InvalidValue).attach_printable(availables);
                return Err(report);
            }
        }
        Ok(result.to_owned())
    }
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
    println!("node: {}", def.data.node_id);
    match client.get_my_native_node(&def.data.node_id).await? {
        Some((id, db)) => {
            if print_diff(&def, &db) {
                if dry_run {
                    return Ok(());
                }
                if !no_confirm {
                    let yes = ask("update node?").await;
                    if !yes {
                        return Ok(());
                    }
                }
                client.update_node(id, &def).await?;
                println!("updated node, id={}", id);
            }
        }
        None => {
            println!("command is not in database");
            if dry_run {
                return Ok(());
            }
            if !no_confirm {
                let yes = ask("upload?").await;
                if !yes {
                    return Ok(());
                }
            }
            let id = client.insert_node(&def).await?;
            println!("inserted new node, id={}", id);
        }
    }

    Ok(())
}

type AsyncStdin = AllowStdIo<BufReader<Stdin>>;

fn stdin() -> AsyncStdin {
    AllowStdIo::new(BufReader::new(std::io::stdin()))
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
            let mut stdin = stdin();
            stdin.read_line(&mut key).await.ok();
            let key = key.trim().to_owned();

            let mut client = ApiClient::new(flow_server, key).await?;
            let username = client.get_username().await?.unwrap_or_default();
            println!("Logged in as {:?}", username);
            client.save_application_data().await?;
        }
        Some(Commands::Node { command }) => match command {
            NodeCommands::New {
                allow_dirty,
                package,
            } => {
                new_node(*allow_dirty, package).await?;
            }
            NodeCommands::Upload {
                path,
                dry_run,
                no_confirm,
            } => {
                upload_node(path, *dry_run, *no_confirm).await?;
            }
        },
        None => {
            Args::command().print_long_help().ok();
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
