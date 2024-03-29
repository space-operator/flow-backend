use command_rpc::client::RpcCommandClient;
use serde::de::value::MapDeserializer;
use std::process::Stdio;
use tempfile::tempdir;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Child,
};
use url::Url;

use super::prelude::*;

#[derive(Deserialize)]
struct Extra {
    source: String,
}

pub async fn new(nd: &NodeData) -> Result<(Box<dyn CommandTrait>, Child), CommandError> {
    let extra = &nd.targets_form.extra.rest;
    let source = Extra::deserialize(MapDeserializer::new(
        extra.iter().map(|(k, v)| (k.as_str(), v)),
    ))?
    .source;

    let dir = tempdir()?;
    tokio::fs::write(dir.path().join("__cmd.ts"), source).await?;
    tokio::fs::write(dir.path().join("__run.ts"), include_str!("./__run.ts")).await?;
    let mut spawned = tokio::process::Command::new("deno")
        .current_dir(dir.path())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .arg("run")
        .arg("--allow-net")
        .arg("__run.ts")
        .spawn()?;
    let mut stdout = BufReader::new(spawned.stdout.take().unwrap()).lines();
    let port = stdout
        .next_line()
        .await?
        .ok_or_else(|| CommandError::msg("port not found"))?
        .parse::<u16>()?;
    let base_url = Url::parse(&format!("http://127.0.0.1:{}", port)).unwrap();
    let cmd = RpcCommandClient::new(base_url, String::new(), nd.clone());
    tokio::spawn(async move {
        while let Ok(Some(line)) = stdout.next_line().await {
            tracing::debug!("{}", line);
        }
    });

    Ok((Box::new(cmd), spawned))
}
