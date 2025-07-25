use anyhow::Context as _;
use command_rpc::client::RpcCommandClient;
use flow_lib::{
    command::{
        CommandDescription, CommandError, CommandTrait, MatchCommand,
        prelude::{Either, async_trait},
    },
    config::client::NodeData,
    utils::LocalBoxFuture,
};
use serde::Deserialize;
use serde::de::value::MapDeserializer;
use std::process::Stdio;
use tempfile::tempdir;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::{Child, Command},
};
use url::Url;

#[derive(Deserialize)]
struct Extra {
    source: String,
}

#[cfg(feature = "local-deps")]
fn copy_dir_all(
    src: impl AsRef<std::path::Path>,
    dst: impl AsRef<std::path::Path>,
) -> std::pin::Pin<Box<impl std::future::Future<Output = std::io::Result<()>>>> {
    Box::pin(async move {
        std::fs::create_dir_all(&dst)?;
        let mut files = tokio::fs::read_dir(src).await?;
        while let Some(entry) = files.next_entry().await? {
            let ty = entry.file_type().await?;
            if ty.is_dir() {
                copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name())).await?;
            } else {
                tokio::fs::copy(entry.path(), dst.as_ref().join(entry.file_name())).await?;
            }
        }
        Ok(())
    })
}

macro_rules! include {
    ($path:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $path))
    };
}

async fn new_owned(nd: NodeData) -> Result<Box<dyn CommandTrait>, CommandError> {
    let extra = &nd.targets_form.extra.rest;
    let source = Extra::deserialize(MapDeserializer::new(
        extra.iter().map(|(k, v)| (k.as_str(), v)),
    ))?
    .source;

    let dir = tempdir()?;

    tokio::fs::write(dir.path().join("cmd.ts"), source)
        .await
        .context("write cmd.ts")?;

    let mut node_data = nd.clone();
    node_data.targets_form.extra.rest.remove("source");
    let node_data_json = serde_json::to_string(&node_data).context("serialize NodeData")?;
    tokio::fs::write(dir.path().join("node-data.json"), node_data_json)
        .await
        .context("write node-data.json")?;

    tokio::fs::write(dir.path().join("run.ts"), include!("/run.ts"))
        .await
        .context("write run.ts")?;

    tokio::fs::write(
        dir.path().join("deps.ts"),
        if cfg!(feature = "local-deps") {
            include!("/deps_local.ts")
        } else {
            include!("/deps_jsr.ts")
        },
    )
    .await
    .context("write deps.ts")?;

    #[cfg(feature = "local-deps")]
    {
        let libs = concat!(env!("CARGO_MANIFEST_DIR"), "/@space-operator");
        copy_dir_all(libs, dir.path().join("@space-operator"))
            .await
            .context("copy dirs")?;
    }

    let deno_dir = std::env::var("DENO_DIR").unwrap_or_else(|_| {
        let mut home = home::home_dir().unwrap();
        home.push(".cache");
        home.push("deno");
        home.display().to_string()
    });

    let mut spawned = Command::new("deno")
        .current_dir(dir.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("DENO_DIR", &deno_dir)
        .env("NO_COLOR", "1")
        .kill_on_drop(true)
        .arg("run")
        .arg("--allow-net")
        .arg("--allow-env=WS_NO_BUFFER_UTIL")
        .arg("--no-prompt")
        .arg("run.ts")
        .spawn()
        .context("spawn")?;

    let mut stdout = BufReader::new(spawned.stdout.take().unwrap()).lines();
    let port = match stdout.next_line().await? {
        Some(line) => line.parse::<u16>().context("parse port")?,
        None => {
            let mut error = String::new();
            let mut stderr = BufReader::new(spawned.stderr.take().unwrap());
            stderr.read_to_string(&mut error).await.map_err(|error| {
                tracing::warn!("read error: {}", error);
                CommandError::msg("could not start command")
            })?;
            return Err(CommandError::msg(error));
        }
    };
    let base_url = Url::parse(&format!("http://127.0.0.1:{port}")).unwrap();
    let cmd = RpcCommandClient::new(base_url, String::new(), node_data.clone());
    let cmd = DenoCommand {
        inner: cmd,
        spawned,
    };
    tokio::spawn(async move {
        while let Ok(Some(line)) = stdout.next_line().await {
            tracing::debug!("{}", line);
        }
    });

    Ok(Box::new(cmd))
}

pub fn new(nd: &NodeData) -> LocalBoxFuture<'static, Result<Box<dyn CommandTrait>, CommandError>> {
    let nd = nd.clone();
    Box::pin(new_owned(nd))
}

flow_lib::submit!(CommandDescription {
    matcher: MatchCommand {
        r#type: flow_lib::CommandType::Deno,
        name: flow_lib::command::MatchName::Regex(std::borrow::Cow::Borrowed("")),
    },
    fn_new: Either::Right(new),
});

pub struct DenoCommand {
    inner: RpcCommandClient,
    spawned: Child,
}

#[async_trait(?Send)]
impl CommandTrait for DenoCommand {
    fn name(&self) -> flow_lib::Name {
        self.inner.name()
    }
    fn inputs(&self) -> Vec<flow_lib::CmdInputDescription> {
        self.inner.inputs()
    }
    fn outputs(&self) -> Vec<flow_lib::CmdOutputDescription> {
        self.inner.outputs()
    }
    fn permissions(&self) -> flow_lib::command::prelude::Permissions {
        self.inner.permissions()
    }
    async fn run(
        &self,
        ctx: flow_lib::context::CommandContext,
        params: flow_lib::ValueSet,
    ) -> Result<flow_lib::value::Map, CommandError> {
        self.inner.run(ctx, params).await
    }
    async fn destroy(&mut self) {
        self.spawned.kill().await.ok();
    }
}

#[cfg(test)]
mod tests {
    use flow_lib::{
        config::{
            client::{Extra, Source, Target, TargetsForm},
            node::Definition,
        },
        context::CommandContext,
    };
    use serde_json::Value as JsonValue;
    use uuid::Uuid;

    use super::*;

    fn node_data(def: &str, source: &str) -> NodeData {
        let def = serde_json::from_str::<Definition>(def).unwrap();
        NodeData {
            r#type: def.r#type,
            node_id: def.data.node_id,
            sources: def
                .sources
                .into_iter()
                .map(|x| Source {
                    id: Uuid::new_v4(),
                    name: x.name,
                    optional: x.optional,
                    r#type: x.r#type,
                })
                .collect(),
            targets: def
                .targets
                .into_iter()
                .map(|x| Target {
                    id: Uuid::new_v4(),
                    name: x.name,
                    required: x.required,
                    passthrough: x.passthrough,
                    type_bounds: x.type_bounds,
                })
                .collect(),
            targets_form: TargetsForm {
                form_data: JsonValue::Null,
                wasm_bytes: None,
                extra: Extra {
                    supabase_id: None,
                    rest: [("source".to_owned(), source.into())].into(),
                },
            },
            instruction_info: None,
        }
    }

    #[actix_web::test]
    async fn test_run() {
        tracing_subscriber::fmt::try_init().ok();
        const SOURCE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/add.ts"));
        const JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/add.json"));
        let nd = node_data(JSON, SOURCE);
        let cmd = new(&nd).await.unwrap();
        let mut ctx = CommandContext::test_context();
        ctx.extensions_mut()
            .unwrap()
            .insert(srpc::Server::start_http_server().unwrap());
        let output = cmd
            .run(ctx, value::map! { "a" => 12, "b" => 13 })
            .await
            .unwrap();
        let c = value::from_value::<f64>(output["c"].clone()).unwrap();
        assert_eq!(c, 25.0);
    }
}
