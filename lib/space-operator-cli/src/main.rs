#![allow(clippy::print_stdout, clippy::print_stderr)]

use cargo_metadata::{Metadata, Package, Target};
use chrono::Utc;
use clap::{ColorChoice, CommandFactory, Parser, Subcommand, ValueEnum};
use directories::ProjectDirs;
use error_stack::{Report, ResultExt};
use futures::{io::AllowStdIo, AsyncBufReadExt};
use postgrest::Postgrest;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use regex::Regex;
use reqwest::{
    header::{HeaderName, HeaderValue, AUTHORIZATION},
    StatusCode,
};
use schema::{CommandDefinition, CommandId, ValueType};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    borrow::{Borrow, Cow},
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

    pub async fn claim_token(
        http: &reqwest::Client,
        flow_server: &Url,
        apikey: &str,
    ) -> Result<Output, Report<Error>> {
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
}

pub mod get_info {
    use url::Url;

    use super::*;

    #[derive(Deserialize, Serialize, Debug)]
    pub struct Output {
        pub supabase_url: Url,
        pub anon_key: String,
    }

    pub async fn get_info(
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
    /// Manage your nodes
    #[command(visible_alias = "n")]
    Node {
        #[command(subcommand)]
        command: NodeCommands,
    },
    /// Generate various things
    #[command(visible_alias = "g")]
    Generate {
        #[command(subcommand)]
        command: GenerateCommands,
    },
}

#[derive(Subcommand, Debug)]
enum GenerateCommands {
    /// Generate input struct
    #[command(visible_alias = "i")]
    Input {
        /// Path to node definition file
        path: PathBuf,
    },
    #[command(visible_alias = "o")]
    /// Generate output struct
    Output {
        /// Path to node definition file
        path: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum NodeCommands {
    /// Generate a new node
    #[command(visible_alias = "n")]
    New {
        /// Allow dirty git repository
        #[arg(long)]
        allow_dirty: bool,
        /// Specify which Rust package to add the new node to
        #[arg(long, short)]
        package: Option<String>,
    },
    /// Upload nodes
    #[command(visible_alias = "n")]
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
    #[error("failed to write file {}", .0.display())]
    WriteFile(PathBuf),
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

async fn read_file(path: impl AsRef<Path>) -> Result<String, Report<Error>> {
    tokio::fs::read_to_string(path.as_ref())
        .await
        .change_context_lazy(|| Error::ReadFile(path.as_ref().to_owned()))
}

async fn write_file(path: impl AsRef<Path>, data: impl AsRef<[u8]>) -> Result<(), Report<Error>> {
    tokio::fs::write(path.as_ref(), data)
        .await
        .change_context_lazy(|| Error::WriteFile(path.as_ref().to_owned()))
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
        let token = claim_token::claim_token(&http, &flow_server, &apikey).await?;
        let info = get_info::get_info(&http, &flow_server, &token.access_token).await?;
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

async fn prompt_node_definition() -> Result<CommandDefinition, Report<Error>> {
    let mut stdin = stdin();

    let name_regex = Regex::new(r#"^[[:alpha:]][[:word:]]*$"#).unwrap();
    let name_hint = "value can only contains characters [a-zA-Z0-9_] and must start with [a-zA-Z]";

    let node_id = Prompt::builder()
        .question("node id: ")
        .check_regex(&name_regex)
        .regex_hint(name_hint)
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
        let type_bound: schema::ValueType = type_bound_str.parse().unwrap();

        let optional = Prompt::builder()
            .question("optional (true/false): ")
            .check_list(&["true", "false"])
            .build()
            .prompt(&mut stdin)
            .await?;
        let optional: bool = optional.parse().unwrap();

        let default_value = if optional && type_bound == ValueType::Bool {
            let default = Prompt::builder()
                .question("default value (empty/true/false): ")
                .check_list(&["true", "false"])
                .allow_empty(true)
                .build()
                .prompt(&mut stdin)
                .await?;
            default
                .parse::<bool>()
                .ok()
                .map(serde_json::Value::Bool)
                .unwrap_or(serde_json::Value::Null)
        } else {
            serde_json::Value::Null
        };

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
            default_value,
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
    let info = if ins {
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
        if !inputs.iter().any(|o| o.name == "submit") {
            println!("adding `submit` input");
            inputs.push(schema::Target {
                name: "submit".to_owned(),
                type_bounds: ["bool".to_owned()].into(),
                default_value: serde_json::Value::Bool(true),
                tooltip: String::new(),
                required: false,
                passthrough: false,
            });
        }

        let info = schema::InstructionInfo {
            before: outputs
                .iter()
                .map(|o| o.name.clone())
                .filter(|name| name != "signature")
                .collect(),
            signature: "signature".to_owned(),
            after: Vec::new(),
        };
        println!(
            "using instruction info: {}",
            serde_json::to_string_pretty(&info).unwrap()
        );
        Some(info)
    } else {
        None
    };

    let def = schema::CommandDefinition {
        r#type: "native".to_owned(),
        data: schema::Data {
            node_definition_version: Some("0.1".to_owned()),
            unique_id: Some(String::new()),
            node_id,
            version: "0.1".to_owned(),
            display_name,
            description,
            tags: Some(Vec::new()),
            related_to: Some(
                [schema::RelatedTo {
                    id: String::new(),
                    r#type: String::new(),
                    relationship: String::new(),
                }]
                .into(),
            ),
            resources: Some(schema::Resources {
                source_code_url: String::new(),
                documentation_url: String::new(),
            }),
            usage: Some(schema::Usage {
                license: "Apache-2.0".to_owned(),
                license_url: String::new(),
                pricing: schema::Pricing {
                    currency: "USDC".to_owned(),
                    purchase_price: 0,
                    price_per_run: 0,
                    custom: Some(schema::CustomPricing {
                        unit: "monthly".to_owned(),
                        value: "0".to_owned(),
                    }),
                },
            }),
            authors: Some(
                [schema::Author {
                    name: "Space Operator".to_owned(),
                    contact: String::new(),
                }]
                .into(),
            ),
            instruction_info: info,
            options: None,
            design: None,
        },
        sources: outputs,
        targets: inputs,
        ui_schema: serde_json::Value::Object(<_>::default()),
        json_schema: serde_json::Value::Object(<_>::default()),
    };

    Ok(def)
}

fn relative_to_pwd<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();
    let mut pwd = std::env::current_dir().unwrap_or_default();
    let mut result = PathBuf::new();
    loop {
        match path.strip_prefix(&pwd) {
            Ok(suffix) => {
                result.push(suffix);
                break;
            }
            Err(_) => {
                pwd.pop();
                result.push("..");
            }
        }
    }
    result
}

async fn write_node_definition(
    def: &CommandDefinition,
    package: &Package,
    modules: &[&str],
) -> Result<Option<PathBuf>, Report<Error>> {
    let root = package
        .manifest_path
        .parent()
        .ok_or_else(|| Report::new(Error::Io("find package path")))?
        .as_std_path();

    let mut path = root.join("node-definitions");
    path.extend(modules);
    tokio::fs::create_dir_all(&path)
        .await
        .change_context(Error::Io("create dir"))?;
    path.push(&format!("{}.json", def.data.node_id));
    let path = relative_to_pwd(path);

    println!("writing node definition to {}", path.display());
    if path.is_file() {
        if !ask("file already exists, overwrite?").await {
            return Ok(None);
        }
    }
    let content = serde_json::to_string_pretty(def).change_context(Error::Json)?;
    write_file(&path, content).await?;
    Ok(Some(path))
}

fn value_type_to_rust_type(ty: schema::ValueType) -> TokenStream {
    match ty {
        schema::ValueType::Bool => quote! { bool },
        schema::ValueType::U8 => quote! { u8 },
        schema::ValueType::U16 => quote! { u16 },
        schema::ValueType::U32 => quote! { u32 },
        schema::ValueType::U64 => quote! { u64},
        schema::ValueType::U128 => quote! { u128 },
        schema::ValueType::I8 => quote! { i8 },
        schema::ValueType::I16 => quote! { i16},
        schema::ValueType::I32 => quote! { i32 },
        schema::ValueType::I64 => quote! { i64 },
        schema::ValueType::I128 => quote! { i128},
        schema::ValueType::F32 => quote! { f32 },
        schema::ValueType::F64 => quote! { f64 },
        schema::ValueType::Decimal => quote! { Decimal },
        schema::ValueType::Pubkey => quote! { Pubkey },
        schema::ValueType::Address => quote! { String },
        schema::ValueType::Keypair => quote! { Keypair },
        schema::ValueType::Signature => quote! { Signature },
        schema::ValueType::String => quote! { String },
        schema::ValueType::Bytes => quote! { Bytes },
        schema::ValueType::Array => quote! { Vec<Value> },
        schema::ValueType::Map => quote! { ValueSet },
        schema::ValueType::Json => quote! { JsonValue },
        schema::ValueType::Free => quote! { Value },
        schema::ValueType::Other => quote! { Value },
    }
}

fn rust_type(
    bounds: &[String],
    optional: bool,
    default_value: &serde_json::Value,
) -> proc_macro2::TokenStream {
    let ty = bounds
        .get(0)
        .and_then(|ty| ty.parse::<ValueType>().ok())
        .unwrap_or(schema::ValueType::Free);
    let use_option =
        optional && !(ty == ValueType::Bool && matches!(default_value, serde_json::Value::Bool(_)));
    let ty = value_type_to_rust_type(ty);
    let ty = if use_option {
        quote! { Option<#ty> }
    } else {
        ty
    };
    ty
}

fn rust_type_serde_decor(
    bounds: &[String],
    optional: bool,
    default_value: &serde_json::Value,
) -> proc_macro2::TokenStream {
    let ty = bounds
        .get(0)
        .and_then(|ty| ty.parse::<ValueType>().ok())
        .unwrap_or(schema::ValueType::Free);
    match ty {
        ValueType::Bool => {
            if optional && default_value.as_bool().is_some() {
                let default = default_value.as_bool().unwrap();
                let path = if default {
                    "value::default::bool_true"
                } else {
                    "value::default::bool_false"
                };
                return quote! { #[serde(default = #path)]};
            }
        }
        ValueType::U8 => {}
        ValueType::U16 => {}
        ValueType::U32 => {}
        ValueType::U64 => {}
        ValueType::U128 => {}
        ValueType::I8 => {}
        ValueType::I16 => {}
        ValueType::I32 => {}
        ValueType::I64 => {}
        ValueType::I128 => {}
        ValueType::F32 => {}
        ValueType::F64 => {}
        ValueType::Decimal => {
            return if optional {
                quote! {
                    #[serde(default, with = "value::decimal::opt")]
                }
            } else {
                quote! {
                    #[serde(with = "value::decimal")]
                }
            };
        }
        ValueType::Pubkey => {
            return if optional {
                quote! {
                    #[serde(default, with = "value::pubkey::opt")]
                }
            } else {
                quote! {
                    #[serde(with = "value::pubkey")]
                }
            };
        }
        ValueType::Address => {}
        ValueType::Keypair => {
            return if optional {
                quote! {
                    #[serde(default, with = "value::keypair::opt")]
                }
            } else {
                quote! {
                    #[serde(with = "value::keypair")]
                }
            };
        }
        ValueType::Signature => {
            return if optional {
                quote! {
                    #[serde(default, with = "value::signature::opt")]
                }
            } else {
                quote! {
                    #[serde(with = "value::signature")]
                }
            };
        }
        ValueType::String => {}
        ValueType::Bytes => {}
        ValueType::Array => {}
        ValueType::Map => {}
        ValueType::Json => {}
        ValueType::Free => {}
        ValueType::Other => {}
    }

    quote! {}
}

fn make_input_struct(
    targets: impl IntoIterator<Item = impl Borrow<schema::Target>>,
) -> TokenStream {
    let inputs = targets.into_iter().map(|t| {
        let t = t.borrow();
        let name = format_ident!("{}", t.name);
        let ty = rust_type(&t.type_bounds, !t.required, &t.default_value);
        let serde_decor = rust_type_serde_decor(&t.type_bounds, !t.required, &t.default_value);
        quote! {
            #serde_decor
            #name: #ty
        }
    });
    quote! {
        #[derive(Deserialize, Serialize, Debug)]
        struct Input {
            #(#inputs),*
        }
    }
}

fn make_output_struct(
    sources: impl IntoIterator<Item = impl Borrow<schema::Source>>,
) -> TokenStream {
    let outputs = sources.into_iter().map(|t| {
        let t = t.borrow();
        let name = format_ident!("{}", t.name);
        let ty = rust_type(&[t.r#type.clone()], t.optional, &t.default_value);
        let serde_decor = rust_type_serde_decor(&[t.r#type.clone()], t.optional, &t.default_value);
        quote! {
            #serde_decor
            #name: #ty
        }
    });
    quote! {
        #[derive(Deserialize, Serialize, Debug)]
        struct Output {
            #(#outputs),*
        }
    }
}

fn fmt_code(code: TokenStream) -> String {
    syn::parse2::<syn::File>(code.clone())
        .map(|file| prettyplease::unparse(&file))
        .unwrap_or_else(|error| {
            eprintln!("invalid code: {}", error);
            code.to_string()
        })
}

fn code_template(def: &CommandDefinition, modules: &[&str]) -> String {
    let node_id = &def.data.node_id;
    let node_definition_path = modules.join("/") + node_id + ".json";
    let input_struct = make_input_struct(&def.targets);
    let output_struct = make_output_struct(&def.sources);
    let code = quote! {
        use flow_lib::command::prelude::*;

        const NAME: &str = #node_id;

        flow_lib::submit!(CommandDescription::new(NAME, |_| build()));

        fn build() -> BuildResult {
            const DEFINITION: &str = flow_lib::node_definition!(#node_definition_path);
            static CACHE: BuilderCache = BuilderCache::new(|| {
                CmdBuilder::new(DEFINITION)?.check_name(NAME)
            });
            Ok(CACHE.clone()?.build(run))
        }

        #input_struct

        #output_struct

        async fn run(mut ctx: Context, input: Input) -> Result<Output, CommandError> {
            Err(CommandError::msg("unimplemented"))
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn test_build() {
                build().unwrap();
            }

            #[tokio::test]
            async fn test_run() {
                let ctx = Context::default();

                build().unwrap().run(ctx, ValueSet::new()).await.unwrap_err();
            }
        }
    };

    fmt_code(code)
}

fn find_parent_module<P: AsRef<Path>>(path: P) -> Result<PathBuf, Report<Error>> {
    let path = path.as_ref();
    let parent = path.parent().ok_or_else(|| Error::Io("get parent path"))?;

    let mod_rs = parent.join("mod.rs");
    if mod_rs.is_file() {
        return Ok(mod_rs);
    }

    let mut mod_rs = parent.to_owned();
    mod_rs.set_extension("rs");
    if mod_rs.is_file() {
        return Ok(mod_rs);
    }

    let lib_rs = parent.join("lib.rs");
    if lib_rs.is_file() {
        return Ok(lib_rs);
    }

    Err(Report::new(Error::Io("find parent module")))
}

async fn update_parent_module(
    def: &CommandDefinition,
    module_path: &Path,
) -> Result<(), Report<Error>> {
    let parent_module_path = find_parent_module(module_path)?;

    let mut parent_module = read_file(&parent_module_path).await?;

    let parsed_parent_module =
        syn::parse_file(&parent_module).change_context(Error::Io("invalid rust code"))?;
    let has_module = parsed_parent_module.items.iter().any(|item| {
        if let syn::Item::Mod(m) = item {
            return m.ident.to_string() == def.data.node_id;
        } else {
            false
        }
    });

    if !has_module {
        std::fmt::write(
            &mut parent_module,
            format_args!("\npub mod {};\n", def.data.node_id),
        )
        .unwrap();
        println!("updating module {}", parent_module_path.display());
        write_file(&parent_module_path, parent_module).await?;
    }
    Ok(())
}

async fn write_code(
    def: &CommandDefinition,
    target: &Target,
    modules: &[&str],
) -> Result<(), Report<Error>> {
    let root = target
        .src_path
        .parent()
        .ok_or_else(|| Report::new(Error::Io("find package source path")))?
        .as_std_path();

    let mut path = root.to_path_buf();
    path.extend(modules);
    tokio::fs::create_dir_all(&path)
        .await
        .change_context(Error::Io("create dir"))?;
    path.push(format!("{}.rs", def.data.node_id));
    let path = relative_to_pwd(path);

    println!("writing code to {}", path.display());
    if path.is_file() {
        if !ask("file already exists, overwrite?").await {
            return Ok(());
        }
    }

    let code = code_template(def, modules);
    tokio::fs::write(&path, code)
        .await
        .change_context(Error::Io("write file"))?;

    if let Err(error) = update_parent_module(def, &path).await {
        eprintln!("failed to update parent module: {:?}", error);
    }

    Ok(())
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
    let lib_target = member
        .targets
        .iter()
        .find(|p| p.is_lib())
        .ok_or_else(|| Error::NotLib(member.name.clone()))?;
    println!("using package: {}", member.name);

    let rust_module_regex = Regex::new(
        r#"^(\p{XID_Start}|_)\p{XID_Continue}*(::(\p{XID_Start}|_)\p{XID_Continue}*)*$"#,
    )
    .unwrap();
    let rust_module_hint = "enter valid Rust module path to save the node (empty to save at root)";
    let module_path = Prompt::builder()
        .question("module path: ")
        .check_regex(&rust_module_regex)
        .regex_hint(rust_module_hint)
        .allow_empty(true)
        .build()
        .prompt(&mut stdin())
        .await?;
    let modules = module_path.split("::").collect::<Vec<_>>();

    let def = prompt_node_definition().await?;

    if let Some(nd) = write_node_definition(&def, member, &modules).await? {
        write_code(&def, lib_target, &modules).await?;

        let upload = ask("upload node").await;
        if upload {
            upload_node(&nd, false, false).await?;
        }
    }
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
                if self.check_list.is_some() || self.regex_hint.is_some() {
                    print!("(?) ");
                }
                print!("{}", self.question);
                std::io::stdout().flush().ok();
                let mut result = String::new();
                stdin.read_line(&mut result).await.ok();
                let result = result.trim();
                if let Some(hint) = &self.regex_hint {
                    if result == "?" {
                        println!("{}", hint);
                        continue;
                    }
                }
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

async fn upload_node(path: &Path, dry_run: bool, no_confirm: bool) -> Result<(), Report<Error>> {
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

async fn generate_input_struct(path: impl AsRef<Path>) -> Result<(), Report<Error>> {
    let nd = read_file(path).await?;
    let nd = serde_json::from_str::<CommandDefinition>(&nd)
        .change_context(Error::Json)
        .attach_printable("not a valid node definition file")?;
    let input_struct = make_input_struct(&nd.targets);
    let code = fmt_code(input_struct);
    println!("{}", code);
    Ok(())
}

async fn generate_output_struct(path: impl AsRef<Path>) -> Result<(), Report<Error>> {
    let nd = read_file(path).await?;
    let nd = serde_json::from_str::<CommandDefinition>(&nd)
        .change_context(Error::Json)
        .attach_printable("not a valid node definition file")?;
    let output_struct = make_output_struct(&nd.sources);
    let code = fmt_code(output_struct);
    println!("{}", code);
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
        Some(Commands::Generate { command }) => match command {
            GenerateCommands::Input { path } => generate_input_struct(path).await?,
            GenerateCommands::Output { path } => generate_output_struct(path).await?,
        },
        None => {
            Args::command().print_long_help().ok();
        }
    }
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let color_mode = match get_color() {
        ColorChoice::Auto => {
            if console::colors_enabled_stderr() {
                error_stack::fmt::ColorMode::Color
            } else {
                error_stack::fmt::ColorMode::None
            }
        }
        ColorChoice::Always => error_stack::fmt::ColorMode::Color,
        ColorChoice::Never => error_stack::fmt::ColorMode::None,
    };
    Report::set_color_mode(color_mode);
    Report::install_debug_hook::<std::panic::Location>(|_, _| {});
    if let Err(error) = run().await {
        eprintln!("error: {:#?}", error);
    }
}
