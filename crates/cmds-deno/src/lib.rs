use anyhow::Context as _;
use command_rpc::client::RpcCommandClient;
use flow_lib::{
    command::{CommandError, CommandTrait},
    config::client::NodeData,
};
use serde::de::value::MapDeserializer;
use serde::Deserialize;
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
async fn copy_dir_all(
    src: impl AsRef<std::path::Path>,
    dst: impl AsRef<std::path::Path>,
) -> std::io::Result<()> {
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
}

macro_rules! include {
    ($path:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $path))
    };
}

pub async fn new(nd: &NodeData) -> Result<(Box<dyn CommandTrait>, Child), CommandError> {
    let extra = &nd.targets_form.extra.rest;
    let source = Extra::deserialize(MapDeserializer::new(
        extra.iter().map(|(k, v)| (k.as_str(), v)),
    ))?
    .source;

    let dir = tempdir()?;

    tokio::fs::write(dir.path().join("cmd.ts"), source)
        .await
        .context("write cmd.ts")?;

    let node_data_json = serde_json::to_string(nd).context("serialize NodeData")?;
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
    let base_url = Url::parse(&format!("http://127.0.0.1:{}", port)).unwrap();
    let cmd = RpcCommandClient::new(base_url, String::new(), nd.clone());
    tokio::spawn(async move {
        while let Ok(Some(line)) = stdout.next_line().await {
            tracing::debug!("{}", line);
        }
    });

    Ok((Box::new(cmd), spawned))
}

// #[cfg(test)]
// mod tests {
//     use flow_lib::{
//         config::client::{Extra, Source, Target, TargetsForm},
//         Context, ValueType,
//     };
//     use serde_json::Value as JsonValue;
//     use std::sync::Arc;
//     use uuid::Uuid;

//     use super::*;

//     #[actix_web::test]
//     async fn test_run() {
//         tracing_subscriber::fmt::try_init().ok();
//         const SOURCE: &str = include_str!(concat!(
//             env!("CARGO_MANIFEST_DIR"),
//             "/test_files/deno_add/add.ts"
//         ));
//         let (cmd, child) = new(&NodeData {
//             r#type: flow_lib::CommandType::Deno,
//             node_id: "my_node".to_owned(),
//             sources: [Source {
//                 id: Uuid::nil(),
//                 name: "c".to_owned(),
//                 r#type: ValueType::F64,
//                 optional: false,
//             }]
//             .into(),
//             targets: [
//                 Target {
//                     id: Uuid::nil(),
//                     name: "a".to_owned(),
//                     type_bounds: [ValueType::F64].into(),
//                     required: true,
//                     passthrough: false,
//                 },
//                 Target {
//                     id: Uuid::nil(),
//                     name: "b".to_owned(),
//                     type_bounds: [ValueType::F64].into(),
//                     required: true,
//                     passthrough: false,
//                 },
//             ]
//             .into(),
//             targets_form: TargetsForm {
//                 form_data: JsonValue::Null,
//                 wasm_bytes: None,
//                 extra: Extra {
//                     supabase_id: None,
//                     rest: [("source".to_owned(), SOURCE.into())].into(),
//                 },
//             },
//         })
//         .await
//         .unwrap();
//         let mut ctx = Context::default();
//         Arc::get_mut(&mut ctx.extensions)
//             .unwrap()
//             .insert(srpc::Server::start_http_server().unwrap());
//         let output = cmd
//             .run(ctx, value::map! { "a" => 12, "b" => 13 })
//             .await
//             .unwrap();
//         let c = value::from_value::<u32>(output["c"].clone()).unwrap();
//         assert_eq!(c, 25);
//         drop(child);
//     }
// }
