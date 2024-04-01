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
        .arg("-A") // TODO
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

#[cfg(test)]
mod tests {
    use flow_lib::config::client::{Extra, Source, Target, TargetsForm};
    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn test_run() {
        tracing_subscriber::fmt::try_init().ok();
        const SOURCE: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_files/deno_add/add.ts"
        ));
        let (cmd, child) = new(&NodeData {
            r#type: flow_lib::CommandType::Deno,
            node_id: "my_node".to_owned(),
            sources: [Source {
                id: Uuid::nil(),
                name: "c".to_owned(),
                r#type: ValueType::F64,
                optional: false,
            }]
            .into(),
            targets: [
                Target {
                    id: Uuid::nil(),
                    name: "a".to_owned(),
                    type_bounds: [ValueType::F64].into(),
                    required: true,
                    passthrough: false,
                },
                Target {
                    id: Uuid::nil(),
                    name: "b".to_owned(),
                    type_bounds: [ValueType::F64].into(),
                    required: true,
                    passthrough: false,
                },
            ]
            .into(),
            targets_form: TargetsForm {
                form_data: JsonValue::Null,
                wasm_bytes: None,
                extra: Extra {
                    supabase_id: None,
                    rest: [("source".to_owned(), SOURCE.into())].into(),
                },
            },
        })
        .await
        .unwrap();
        let mut ctx = Context::default();
        Arc::get_mut(&mut ctx.extensions)
            .unwrap()
            .insert(srpc::Server::start_http_server().unwrap());
        let output = cmd
            .run(ctx, value::map! { "a" => 12, "b" => 13 })
            .await
            .unwrap();
        let c = value::from_value::<u32>(output["c"].clone()).unwrap();
        assert_eq!(c, 25);
        drop(child);
    }
}
