//! Bun-based command runner.
//!
//! Parallel to `cmds-deno` — spawns Bun subprocesses instead of Deno.
//! Uses the same HTTP RPC protocol (POST /call) so the Rust side is identical.
//! Pre-built nodes get their TS source embedded via `include_str!`.

use anyhow::Context as _;
use flow_lib::{
    command::{CommandError, CommandTrait, default_node_data, prelude::async_trait},
    config::client::{self, NodeData},
    utils::LocalBoxFuture,
};
use flow_rpc::client::RpcCommandClient;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tempfile::{TempDir, tempdir};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::{Child, Command},
    sync::Mutex as AsyncMutex,
};
use tokio_util::sync::CancellationToken;
use url::Url;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

const STDERR_TAIL_LIMIT: usize = 64;
const DEFAULT_BUN_STARTUP_TIMEOUT_SECS: u64 = 30;
const DEFAULT_BUN_RUN_TIMEOUT_SECS: u64 = 180;
const BUN_ENV_ALLOWLIST: &[&str] = &[
    "ALL_PROXY",
    "HOME",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "LANG",
    "LC_ALL",
    "LOGNAME",
    "NODE_EXTRA_CA_CERTS",
    "NO_PROXY",
    "PATH",
    "SSL_CERT_DIR",
    "SSL_CERT_FILE",
    "TEMP",
    "TMP",
    "TMPDIR",
    "TZ",
    "USER",
];

fn source_from_config(nd: &NodeData) -> Option<String> {
    ["source", "code"].into_iter().find_map(|key| {
        nd.config.get(key).and_then(|json| {
            match flow_lib::command::parse_value_tagged_or_json(json.clone()) {
                flow_lib::value::Value::String(s) => Some(s),
                _ => None,
            }
        })
    })
}

macro_rules! embed {
    ($path:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $path))
    };
}

fn find_node_modules_from(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|dir| {
        let candidate = dir.join("node_modules");
        candidate.exists().then_some(candidate)
    })
}

fn workspace_node_modules_dir() -> Option<PathBuf> {
    find_node_modules_from(Path::new(env!("CARGO_MANIFEST_DIR"))).or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|dir| find_node_modules_from(&dir))
    })
}

fn symlink_workspace_node_modules(dst: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(workspace_node_modules_dir().unwrap(), dst)
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(workspace_node_modules_dir().unwrap(), dst)
    }
}

fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name),
        Ok(value)
            if matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
    )
}

fn duration_from_env(name: &str, default_secs: u64) -> Duration {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(default_secs))
}

fn bun_startup_timeout() -> Duration {
    duration_from_env(
        "SPACE_OPERATOR_BUN_STARTUP_TIMEOUT_SECS",
        DEFAULT_BUN_STARTUP_TIMEOUT_SECS,
    )
}

fn bun_run_timeout() -> Duration {
    duration_from_env(
        "SPACE_OPERATOR_BUN_TIMEOUT_SECS",
        DEFAULT_BUN_RUN_TIMEOUT_SECS,
    )
}

fn copy_allowed_environment(command: &mut Command) {
    command.env_clear();
    for key in BUN_ENV_ALLOWLIST {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command.env("NO_COLOR", "1");
}

fn configure_bun_process(command: &mut Command) {
    #[cfg(unix)]
    command.process_group(0);
}

fn push_stderr_line(stderr_tail: &Arc<Mutex<VecDeque<String>>>, line: String) {
    let mut stderr_tail = stderr_tail.lock().unwrap();
    if stderr_tail.len() >= STDERR_TAIL_LIMIT {
        stderr_tail.pop_front();
    }
    stderr_tail.push_back(line);
}

fn stderr_tail_text(stderr_tail: &Arc<Mutex<VecDeque<String>>>) -> Option<String> {
    let stderr_tail = stderr_tail.lock().unwrap();
    (!stderr_tail.is_empty()).then(|| stderr_tail.iter().cloned().collect::<Vec<_>>().join("\n"))
}

fn exit_status_summary(status: std::process::ExitStatus) -> String {
    if let Some(code) = status.code() {
        return format!("exit code {code}");
    }

    #[cfg(unix)]
    if let Some(signal) = status.signal() {
        return format!("signal {signal}");
    }

    "unknown exit status".to_owned()
}

fn runtime_failure_details(
    started_at: Instant,
    status: std::process::ExitStatus,
    stderr_tail: &Arc<Mutex<VecDeque<String>>>,
) -> String {
    let elapsed_ms = started_at.elapsed().as_millis();
    let mut details = format!(
        "bun subprocess exited after {elapsed_ms}ms ({})",
        exit_status_summary(status)
    );
    if let Some(stderr) = stderr_tail_text(stderr_tail) {
        details.push_str("\nlast bun stderr:\n");
        details.push_str(&stderr);
    }
    details
}

async fn terminate_child_process(child: &mut Child) -> std::io::Result<std::process::ExitStatus> {
    #[cfg(unix)]
    {
        if let Some(pid) = child.id() {
            let result = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
            if result == -1 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(error);
                }
            }
        } else {
            child.start_kill()?;
        }
        child.wait().await
    }

    #[cfg(not(unix))]
    {
        child.start_kill()?;
        child.wait().await
    }
}

/// Known companion modules that may be imported by cmd.ts via relative paths.
/// Maps the import specifier to the embedded source.
const COMPANION_MODULES: &[(&str, &str)] = &[
    (
        "./umbra_common.ts",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/umbra/umbra_common.ts"
        )),
    ),
    (
        "./privacy_cash_common.ts",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/privacy_cash/privacy_cash_common.ts"
        )),
    ),
    (
        "./relay_common.ts",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/relay/relay_common.ts"
        )),
    ),
];

/// If cmd.ts imports any known companion modules, write them into the temp dir.
async fn write_companion_modules(
    dir: &Path,
    source: &str,
) -> Result<(), flow_lib::command::CommandError> {
    for (specifier, content) in COMPANION_MODULES {
        if source.contains(specifier) {
            let filename = specifier.strip_prefix("./").unwrap_or(specifier);
            tokio::fs::write(dir.join(filename), content)
                .await
                .context(format!("write companion {filename}"))?;
        }
    }
    Ok(())
}

fn sanitized_node_data(mut nd: NodeData) -> NodeData {
    if let Some(obj) = nd.config.as_object_mut() {
        obj.remove("code");
        obj.remove("source");
    }
    nd
}

async fn startup_failure_details(mut child: Child, summary: String) -> CommandError {
    let status = terminate_child_process(&mut child).await.ok();

    let mut stderr = String::new();
    if let Some(pipe) = child.stderr.take() {
        let mut reader = BufReader::new(pipe);
        if let Err(error) = reader.read_to_string(&mut stderr).await {
            tracing::warn!("read error: {}", error);
        }
    }

    let mut message = summary;
    if let Some(status) = status {
        message.push_str(&format!(" ({})", exit_status_summary(status)));
    }
    let stderr = stderr.trim();
    if !stderr.is_empty() {
        message.push('\n');
        message.push_str(stderr);
    }
    CommandError::msg(message)
}

struct RunningBun {
    base_url: Url,
    child: Child,
    started_at: Instant,
    stderr_tail: Arc<Mutex<VecDeque<String>>>,
    _dir: TempDir,
}

async fn spawn_running_bun(source: &str, node_data: &NodeData) -> Result<RunningBun, CommandError> {
    let dir = tempdir()?;

    tokio::fs::write(dir.path().join("cmd.ts"), source)
        .await
        .context("write cmd.ts")?;

    write_companion_modules(dir.path(), source).await?;

    let node_data_json = serde_json::to_string(node_data).context("serialize NodeData")?;
    tokio::fs::write(dir.path().join("node-data.json"), node_data_json)
        .await
        .context("write node-data.json")?;

    tokio::fs::write(dir.path().join("run.ts"), embed!("/run.ts"))
        .await
        .context("write run.ts")?;

    tokio::fs::write(dir.path().join("package.json"), embed!("/package.json"))
        .await
        .context("write package.json")?;

    if workspace_node_modules_dir().is_none() {
        return Err(CommandError::msg(
            "could not find workspace node_modules for Bun command; run `bun install` in the repo root",
        ));
    }
    symlink_workspace_node_modules(&dir.path().join("node_modules"))
        .context("symlink workspace node_modules")?;

    let use_smol = env_flag("SPACE_OPERATOR_BUN_SMOL");

    let mut bun = Command::new("bun");
    configure_bun_process(&mut bun);
    copy_allowed_environment(&mut bun);
    if use_smol {
        tracing::info!("starting bun subprocess with --smol");
        bun.arg("--smol");
    }

    let mut spawned = bun
        .current_dir(dir.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .arg("run")
        .arg("run.ts")
        .spawn()
        .context("spawn bun")?;

    let startup_timeout = bun_startup_timeout();
    let mut stdout = BufReader::new(spawned.stdout.take().unwrap()).lines();
    let port = match tokio::time::timeout(startup_timeout, stdout.next_line()).await {
        Ok(line) => match line? {
            Some(line) => line.parse::<u16>().context("parse port")?,
            None => {
                return Err(startup_failure_details(
                    spawned,
                    "could not start bun command".to_owned(),
                )
                .await);
            }
        },
        Err(_) => {
            return Err(startup_failure_details(
                spawned,
                format!(
                    "timed out waiting {}s for bun command startup",
                    startup_timeout.as_secs()
                ),
            )
            .await);
        }
    };
    let started_at = Instant::now();

    let stderr_tail = Arc::new(Mutex::new(VecDeque::new()));
    let stderr_tail_reader = Arc::clone(&stderr_tail);
    let stderr = spawned.stderr.take().unwrap();
    tokio::spawn(async move {
        let mut stderr = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = stderr.next_line().await {
            tracing::warn!("{}", line);
            push_stderr_line(&stderr_tail_reader, line);
        }
    });

    tokio::spawn(async move {
        while let Ok(Some(line)) = stdout.next_line().await {
            tracing::debug!("{}", line);
        }
    });

    Ok(RunningBun {
        base_url: Url::parse(&format!("http://127.0.0.1:{port}")).unwrap(),
        child: spawned,
        started_at,
        stderr_tail,
        _dir: dir,
    })
}

async fn terminate_running_bun(state: &mut RunningBun, reason: &str) -> String {
    match terminate_child_process(&mut state.child).await {
        Ok(status) => format!(
            "{reason}\n{}",
            runtime_failure_details(state.started_at, status, &state.stderr_tail)
        ),
        Err(error) => {
            let mut details = format!("{reason}\nfailed to terminate bun subprocess: {error}");
            if let Some(stderr) = stderr_tail_text(&state.stderr_tail) {
                details.push_str("\nlast bun stderr:\n");
                details.push_str(&stderr);
            }
            details
        }
    }
}

pub(crate) async fn new_owned(nd: NodeData) -> Result<Box<dyn CommandTrait>, CommandError> {
    let source = source_from_config(&nd)
        .ok_or_else(|| CommandError::msg("bun command source/code not found"))?;
    Ok(Box::new(BunCommand {
        node_data: sanitized_node_data(nd),
        source,
        running: AsyncMutex::new(None),
    }))
}

pub fn new(nd: &NodeData) -> LocalBoxFuture<'static, Result<Box<dyn CommandTrait>, CommandError>> {
    let nd = nd.clone();
    Box::pin(new_owned(nd))
}

/// Build a compiled Bun command from embedded `.jsonc` definition and `.ts` source.
/// Called by the auto-generated code from build.rs.
pub fn make_compiled_bun(
    def_jsonc: &str,
    ts_source: &'static str,
    nd: &NodeData,
) -> LocalBoxFuture<'static, Result<Box<dyn CommandTrait>, CommandError>> {
    let def = match flow_lib::config::node::parse_definition(def_jsonc) {
        Ok(d) => d,
        Err(e) => {
            return Box::pin(
                async move { Err(CommandError::msg(format!("parse definition: {e}"))) },
            );
        }
    };

    let mut node_data = NodeData {
        r#type: def.r#type,
        node_id: def.data.node_id,
        outputs: def
            .outputs
            .into_iter()
            .map(|x| client::OutputPort {
                id: uuid::Uuid::new_v4(),
                name: x.name,
                optional: x.optional,
                r#type: x.r#type,
                tooltip: None,
            })
            .collect(),
        inputs: def
            .inputs
            .into_iter()
            .map(|x| client::InputPort {
                id: uuid::Uuid::new_v4(),
                name: x.name,
                required: x.required,
                passthrough: x.passthrough,
                type_bounds: x.type_bounds,
                tooltip: None,
            })
            .collect(),
        config: serde_json::json!({ "source": ts_source }),
        wasm: None,
        instruction_info: None,
    };

    // Merge runtime config values from the flow
    if let Some(obj) = nd.config.as_object() {
        if let Some(target) = node_data.config.as_object_mut() {
            for (k, v) in obj {
                if k != "source" && k != "code" {
                    target.insert(k.clone(), v.clone());
                }
            }
        }
    }

    Box::pin(new_owned(node_data))
}

// Auto-generated node registrations from build.rs
// Scans node-definitions/**/*.jsonc + *.ts pairs
include!(concat!(env!("OUT_DIR"), "/bun_nodes_generated.rs"));

pub struct BunCommand {
    node_data: NodeData,
    source: String,
    running: AsyncMutex<Option<RunningBun>>,
}

enum BunRunOutcome {
    Success(flow_lib::value::Map),
    Failure {
        error: CommandError,
        diagnostics: Option<String>,
        reset_process: bool,
    },
    Timeout {
        details: String,
    },
    Canceled {
        details: String,
    },
}

impl BunCommand {
    async fn ensure_running(
        &self,
        running: &mut Option<RunningBun>,
    ) -> Result<(), flow_lib::command::CommandError> {
        let needs_spawn = match running.as_mut() {
            Some(state) => match state.child.try_wait() {
                Ok(Some(status)) => {
                    tracing::warn!(
                        "{}",
                        runtime_failure_details(state.started_at, status, &state.stderr_tail)
                    );
                    true
                }
                Ok(None) => false,
                Err(error) => {
                    tracing::warn!("bun subprocess status check failed: {}", error);
                    true
                }
            },
            None => true,
        };

        if needs_spawn {
            *running = Some(spawn_running_bun(&self.source, &self.node_data).await?);
        }

        Ok(())
    }
}

#[async_trait(?Send)]
impl CommandTrait for BunCommand {
    fn r#type(&self) -> flow_lib::CommandType {
        flow_lib::CommandType::Bun
    }
    fn name(&self) -> flow_lib::Name {
        self.node_data.node_id.clone()
    }
    fn inputs(&self) -> Vec<flow_lib::CmdInputDescription> {
        self.node_data.cmd_inputs()
    }
    fn outputs(&self) -> Vec<flow_lib::CmdOutputDescription> {
        self.node_data.cmd_outputs()
    }
    async fn run(
        &self,
        ctx: flow_lib::context::CommandContext,
        params: flow_lib::ValueSet,
    ) -> Result<flow_lib::value::Map, CommandError> {
        let cancel_token = ctx.get::<CancellationToken>().cloned();
        let timeout = bun_run_timeout();
        let mut running = self.running.lock().await;
        self.ensure_running(&mut running).await?;

        let outcome = {
            let state = running
                .as_mut()
                .expect("ensure_running always populates the Bun runtime");
            let client = RpcCommandClient::new(
                state.base_url.clone(),
                String::new(),
                self.node_data.clone(),
            );
            let run = client.run(ctx, params);
            tokio::pin!(run);

            if let Some(cancel_token) = cancel_token {
                tokio::select! {
                    _ = cancel_token.cancelled() => BunRunOutcome::Canceled {
                        details: terminate_running_bun(state, "bun command canceled").await,
                    },
                    result = tokio::time::timeout(timeout, &mut run) => match result {
                        Ok(Ok(output)) => BunRunOutcome::Success(output),
                        Ok(Err(error)) => {
                            let (diagnostics, reset_process) = match state.child.try_wait() {
                                Ok(Some(status)) => (
                                    Some(runtime_failure_details(
                                        state.started_at,
                                        status,
                                        &state.stderr_tail,
                                    )),
                                    true,
                                ),
                                Ok(None) => (None, false),
                                Err(wait_error) => (
                                    Some(format!(
                                        "bun subprocess status check failed after {}ms: {}",
                                        state.started_at.elapsed().as_millis(),
                                        wait_error
                                    )),
                                    true,
                                ),
                            };
                            BunRunOutcome::Failure {
                                error,
                                diagnostics,
                                reset_process,
                            }
                        }
                        Err(_) => BunRunOutcome::Timeout {
                            details: terminate_running_bun(
                                state,
                                &format!("bun command timed out after {}s", timeout.as_secs()),
                            )
                            .await,
                        },
                    }
                }
            } else {
                match tokio::time::timeout(timeout, &mut run).await {
                    Ok(Ok(output)) => BunRunOutcome::Success(output),
                    Ok(Err(error)) => {
                        let (diagnostics, reset_process) = match state.child.try_wait() {
                            Ok(Some(status)) => (
                                Some(runtime_failure_details(
                                    state.started_at,
                                    status,
                                    &state.stderr_tail,
                                )),
                                true,
                            ),
                            Ok(None) => (None, false),
                            Err(wait_error) => (
                                Some(format!(
                                    "bun subprocess status check failed after {}ms: {}",
                                    state.started_at.elapsed().as_millis(),
                                    wait_error
                                )),
                                true,
                            ),
                        };
                        BunRunOutcome::Failure {
                            error,
                            diagnostics,
                            reset_process,
                        }
                    }
                    Err(_) => BunRunOutcome::Timeout {
                        details: terminate_running_bun(
                            state,
                            &format!("bun command timed out after {}s", timeout.as_secs()),
                        )
                        .await,
                    },
                }
            }
        };

        match outcome {
            BunRunOutcome::Success(output) => Ok(output),
            BunRunOutcome::Failure {
                error,
                diagnostics,
                reset_process,
            } => {
                if reset_process {
                    running.take();
                }
                let message = diagnostics.map_or_else(
                    || error.to_string(),
                    |details| format!("{error}\n{details}"),
                );
                Err(CommandError::msg(message))
            }
            BunRunOutcome::Timeout { details } | BunRunOutcome::Canceled { details } => {
                running.take();
                Err(CommandError::msg(details))
            }
        }
    }
    async fn destroy(&mut self) {
        if let Some(mut state) = self.running.get_mut().take() {
            terminate_child_process(&mut state.child).await.ok();
        }
    }
    fn node_data(&self) -> client::NodeData {
        let mut data = default_node_data(self);
        if let Some(obj) = data.config.as_object_mut() {
            obj.insert("source".to_owned(), self.source.clone().into());
        }
        data
    }
}

#[cfg(test)]
pub mod test_utils;

#[cfg(test)]
mod tests {
    use super::*;
    use flow_lib::{
        config::{
            client::{InputPort, OutputPort},
            node::parse_definition,
        },
        context::{CommandContext, FlowServices, FlowSetServices, execute, get_jwt, signer},
        flow_run_events,
        solana::{Instructions, Pubkey, Signature, Wallet},
        utils::tower_client::unimplemented_svc,
    };
    use solana_keypair::Signer;
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };
    use tempfile::tempdir;
    use uuid::Uuid;

    fn node_data(def: &str, source: &str) -> NodeData {
        let def = parse_definition(def).unwrap();
        NodeData {
            r#type: def.r#type,
            node_id: def.data.node_id,
            outputs: def
                .outputs
                .into_iter()
                .map(|x| OutputPort {
                    id: Uuid::new_v4(),
                    name: x.name,
                    optional: x.optional,
                    r#type: x.r#type,
                    tooltip: None,
                })
                .collect(),
            inputs: def
                .inputs
                .into_iter()
                .map(|x| InputPort {
                    id: Uuid::new_v4(),
                    name: x.name,
                    required: x.required,
                    passthrough: x.passthrough,
                    type_bounds: x.type_bounds,
                    tooltip: None,
                })
                .collect(),
            config: serde_json::json!({ "source": source }),
            wasm: None,
            instruction_info: None,
        }
    }

    fn test_context(execute_svc: execute::Svc, signer_svc: signer::Svc) -> CommandContext {
        let base = CommandContext::test_context();
        let data = base.raw().data.clone();
        let node_id = data.node_id;
        let times = data.times;
        let (tx, _) = flow_run_events::channel();

        CommandContext::builder()
            .execute(execute_svc)
            .get_jwt(unimplemented_svc::<
                get_jwt::Request,
                get_jwt::Response,
                get_jwt::Error,
            >())
            .flow(FlowServices {
                signer: signer_svc,
                set: FlowSetServices {
                    http: base.http().clone(),
                    solana_client: base.solana_client().clone(),
                    helius: None,
                    extensions: Default::default(),
                    api_input: unimplemented_svc(),
                },
            })
            .data(data)
            .node_log(flow_run_events::NodeLogSender::new(tx, node_id, times))
            .build()
    }

    #[test]
    fn finds_workspace_node_modules_in_ancestor() {
        let tmp = tempdir().unwrap();
        let workspace = tmp.path().join("workspace");
        let crate_dir = workspace.join("crates/cmds-bun");
        std::fs::create_dir_all(&crate_dir).unwrap();
        std::fs::create_dir_all(workspace.join("node_modules")).unwrap();

        let found = find_node_modules_from(&crate_dir).unwrap();
        assert_eq!(found, workspace.join("node_modules"));
    }

    #[actix_web::test]
    async fn test_run() {
        tracing_subscriber::fmt::try_init().ok();
        const SOURCE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/add.ts"));
        const JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/add.jsonc"));

        let nd = node_data(JSON, SOURCE);
        let cmd = new(&nd).await.unwrap();
        let ctx = test_utils::test_context();
        let output = cmd
            .run(ctx, value::map! { "a" => 12, "b" => 13 })
            .await
            .unwrap();
        let c = value::from_value::<f64>(output["c"].clone()).unwrap();
        assert_eq!(c, 25.0);
    }

    #[actix_web::test]
    async fn test_ctx_kv_reports_migration_guidance() {
        tracing_subscriber::fmt::try_init().ok();

        const JSON: &str = r#"{
          "version": "0.1",
          "name": "kv_unavailable",
          "prefix": "bun",
          "type": "bun",
          "author_handle": "spo",
          "ports": {
            "inputs": [],
            "outputs": []
          },
          "config_schema": {},
          "config": {}
        }"#;
        const SOURCE: &str = r#"
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";

export default class KvUnavailable extends BaseCommand {
  override async run(ctx: Context): Promise<Record<string, never>> {
    await ctx.kv.set("foo", "bar");
    return {};
  }
}
"#;

        let ctx = test_utils::test_context();
        let cmd = new(&node_data(JSON, SOURCE)).await.unwrap();
        let err = cmd.run(ctx, value::map! {}).await.unwrap_err();
        assert!(
            err.to_string()
                .contains("ctx.kv is not available in script runtimes"),
            "{err:?}"
        );
    }

    #[actix_web::test]
    async fn test_execute_with_public_key_signer_uses_adapter_wallet() {
        tracing_subscriber::fmt::try_init().ok();

        const JSON: &str = r#"{
          "version": "0.1",
          "name": "adapter_execute",
          "prefix": "bun",
          "type": "bun",
          "author_handle": "spo",
          "ports": {
            "inputs": [
              { "name": "signer", "type_bounds": ["string"], "required": true, "passthrough": false }
            ],
            "outputs": [
              { "name": "ok", "type": "f64", "optional": false }
            ]
          },
          "config_schema": {},
          "config": {}
        }"#;
        const SOURCE: &str = r#"
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { Instructions } from "@space-operator/flow-lib-bun/context";
import { PublicKey } from "@solana/web3.js";

export default class AdapterExecute extends BaseCommand {
  override async run(ctx: Context, inputs: { signer: string }): Promise<{ ok: number }> {
    const signer = new PublicKey(inputs.signer);
    await ctx.execute(new Instructions(signer, [signer], []), {});
    return { ok: 1 };
  }
}
"#;

        let observed = Arc::new(Mutex::new(None::<Instructions>));
        let execute_svc = execute::Svc::new(tower::service_fn({
            let observed = observed.clone();
            move |req: execute::Request| {
                let observed = observed.clone();
                async move {
                    *observed.lock().unwrap() = Some(req.instructions.clone());
                    Ok(execute::Response {
                        signature: Some(Signature::default()),
                    })
                }
            }
        }));

        let mut ctx = test_context(
            execute_svc,
            unimplemented_svc::<signer::SignatureRequest, signer::SignatureResponse, signer::Error>(
            ),
        );
        ctx.extensions_mut()
            .unwrap()
            .insert(tower_rpc::Server::start_http_server().unwrap());

        let signer = Pubkey::new_unique();
        let cmd = new(&node_data(JSON, SOURCE)).await.unwrap();
        let output = cmd
            .run(ctx, value::map! { "signer" => signer.to_string() })
            .await
            .unwrap();

        assert_eq!(value::from_value::<f64>(output["ok"].clone()).unwrap(), 1.0);

        let instructions = observed.lock().unwrap().clone().unwrap();
        assert_eq!(instructions.fee_payer, signer);
        assert_eq!(
            instructions.signers,
            vec![Wallet::Adapter {
                public_key: signer,
                token: None,
            }]
        );
    }

    #[actix_web::test]
    async fn test_request_signature_uses_signer_service() {
        tracing_subscriber::fmt::try_init().ok();

        const JSON: &str = r#"{
          "version": "0.1",
          "name": "adapter_signature",
          "prefix": "bun",
          "type": "bun",
          "author_handle": "spo",
          "ports": {
            "inputs": [
              { "name": "signer", "type_bounds": ["string"], "required": true, "passthrough": false }
            ],
            "outputs": [
              { "name": "signature_length", "type": "f64", "optional": false },
              { "name": "new_message_length", "type": "f64", "optional": false }
            ]
          },
          "config_schema": {},
          "config": {}
        }"#;
        const SOURCE: &str = r#"
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { PublicKey } from "@solana/web3.js";

export default class AdapterSignature extends BaseCommand {
  override async run(
    ctx: Context,
    inputs: { signer: string },
  ): Promise<{ signature_length: number; new_message_length: number }> {
    const signer = new PublicKey(inputs.signer);
    const { signature, new_message } = await ctx.requestSignature(
      signer,
      new Uint8Array([1, 2, 3, 4]),
    );
    return {
      signature_length: signature.length,
      new_message_length: new_message ? new_message.length : 0,
    };
  }
}
"#;

        let observed = Arc::new(Mutex::new(None::<signer::SignatureRequest>));
        let signer_svc = signer::Svc::new(tower::service_fn({
            let observed = observed.clone();
            move |req: signer::SignatureRequest| {
                let observed = observed.clone();
                async move {
                    *observed.lock().unwrap() = Some(req);
                    Ok(signer::SignatureResponse {
                        signature: Signature::from([7u8; 64]),
                        new_message: Some(b"updated".to_vec().into()),
                    })
                }
            }
        }));

        let mut ctx = test_context(
            unimplemented_svc::<execute::Request, execute::Response, execute::Error>(),
            signer_svc,
        );
        ctx.extensions_mut()
            .unwrap()
            .insert(tower_rpc::Server::start_http_server().unwrap());

        let signer = Pubkey::new_unique();
        let cmd = new(&node_data(JSON, SOURCE)).await.unwrap();
        let output = cmd
            .run(ctx, value::map! { "signer" => signer.to_string() })
            .await
            .unwrap();

        assert_eq!(
            value::from_value::<f64>(output["signature_length"].clone()).unwrap(),
            64.0
        );
        assert_eq!(
            value::from_value::<f64>(output["new_message_length"].clone()).unwrap(),
            7.0
        );

        let request = observed.lock().unwrap().clone().unwrap();
        assert_eq!(request.pubkey, signer);
        assert_eq!(request.message.as_ref(), b"\x01\x02\x03\x04");
        assert_eq!(request.timeout, Duration::from_secs(120));
        assert_eq!(
            request.kind,
            signer::SignatureRequestKind::TransactionMessage
        );
    }

    #[actix_web::test]
    async fn test_request_message_signature_uses_signer_service() {
        tracing_subscriber::fmt::try_init().ok();

        const JSON: &str = r#"{
          "version": "0.1",
          "name": "adapter_message_signature",
          "prefix": "bun",
          "type": "bun",
          "author_handle": "spo",
          "ports": {
            "inputs": [
              { "name": "signer", "type_bounds": ["string"], "required": true, "passthrough": false }
            ],
            "outputs": [
              { "name": "signature_length", "type": "f64", "optional": false }
            ]
          },
          "config_schema": {},
          "config": {}
        }"#;
        const SOURCE: &str = r#"
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { PublicKey } from "@solana/web3.js";

export default class AdapterMessageSignature extends BaseCommand {
  override async run(
    ctx: Context,
    inputs: { signer: string },
  ): Promise<{ signature_length: number }> {
    const signer = new PublicKey(inputs.signer);
    const { signature } = await ctx.requestMessageSignature(
      signer,
      new Uint8Array([9, 8, 7, 6]),
    );
    return {
      signature_length: signature.length,
    };
  }
}
"#;

        let observed = Arc::new(Mutex::new(None::<signer::SignatureRequest>));
        let signer_svc = signer::Svc::new(tower::service_fn({
            let observed = observed.clone();
            move |req: signer::SignatureRequest| {
                let observed = observed.clone();
                async move {
                    *observed.lock().unwrap() = Some(req);
                    Ok(signer::SignatureResponse {
                        signature: Signature::from([9u8; 64]),
                        new_message: None,
                    })
                }
            }
        }));

        let mut ctx = test_context(
            unimplemented_svc::<execute::Request, execute::Response, execute::Error>(),
            signer_svc,
        );
        ctx.extensions_mut()
            .unwrap()
            .insert(tower_rpc::Server::start_http_server().unwrap());

        let signer = Pubkey::new_unique();
        let cmd = new(&node_data(JSON, SOURCE)).await.unwrap();
        let output = cmd
            .run(ctx, value::map! { "signer" => signer.to_string() })
            .await
            .unwrap();

        assert_eq!(
            value::from_value::<f64>(output["signature_length"].clone()).unwrap(),
            64.0
        );

        let request = observed.lock().unwrap().clone().unwrap();
        assert_eq!(request.pubkey, signer);
        assert_eq!(request.message.as_ref(), b"\x09\x08\x07\x06");
        assert_eq!(request.timeout, Duration::from_secs(120));
        assert_eq!(request.kind, signer::SignatureRequestKind::Message);
    }

    /// Spawn a compiled bun node from its .jsonc + .ts pair under node-definitions/ and src/.
    async fn spawn_umbra_node(name: &str) -> Box<dyn CommandTrait> {
        let jsonc = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("node-definitions/umbra")
                .join(format!("{name}.jsonc")),
        )
        .unwrap();
        let ts_source: &str = match name {
            "umbra_register" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/umbra/umbra_register.ts"
                ))
            }
            "umbra_query_account" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/umbra/umbra_query_account.ts"
                ))
            }
            "umbra_query_balance" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/umbra/umbra_query_balance.ts"
                ))
            }
            "umbra_deposit" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/umbra/umbra_deposit.ts"
                ))
            }
            "umbra_withdraw" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/umbra/umbra_withdraw.ts"
                ))
            }
            "umbra_fetch_utxos" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/umbra/umbra_fetch_utxos.ts"
                ))
            }
            "umbra_create_utxo" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/umbra/umbra_create_utxo.ts"
                ))
            }
            "umbra_claim_utxo" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/umbra/umbra_claim_utxo.ts"
                ))
            }
            _ => panic!("unknown umbra node: {name}"),
        };
        let nd = node_data(&jsonc, ts_source);
        make_compiled_bun(&jsonc, ts_source, &nd).await.unwrap()
    }

    // ── Solana integration tests ───────────────────────────────────────

    /// Spawn a compiled solana node from its .jsonc + .ts pair.
    async fn spawn_solana_node(name: &str) -> Box<dyn CommandTrait> {
        let jsonc = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("node-definitions/solana")
                .join(format!("{name}.jsonc")),
        )
        .unwrap();
        let ts_source: &str = match name {
            "transfer_sol" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/solana/transfer_sol.ts"
                ))
            }
            _ => panic!("unknown solana node: {name}"),
        };
        let nd = node_data(&jsonc, ts_source);
        make_compiled_bun(&jsonc, ts_source, &nd).await.unwrap()
    }

    #[actix_web::test]
    #[ignore = "requires funded devnet wallet: TEST_WALLET_KEYPAIR"]
    async fn test_transfer_sol_devnet() {
        tracing_subscriber::fmt::try_init().ok();

        let cmd = spawn_solana_node("transfer_sol").await;
        let ctx = test_utils::test_context();

        let sender = test_utils::test_wallet();
        let sender_kp = sender.keypair().unwrap();
        let recipient = solana_keypair::Keypair::new();

        let rpc = solana_rpc_client::nonblocking::rpc_client::RpcClient::new(
            "https://api.devnet.solana.com".to_string(),
        );
        test_utils::ensure_funded(&rpc, &sender_kp.pubkey(), 0.1).await;

        let output = cmd
            .run(
                ctx,
                value::map! {
                    "sender" => sender_kp.to_bytes(),
                    "recipient" => recipient.pubkey().to_bytes(),
                    "amount" => 1_000_000u64, // 0.001 SOL
                },
            )
            .await
            .unwrap();

        assert!(
            output.contains_key("signature"),
            "expected signature in output"
        );
    }

    // ── Umbra integration tests (mainnet, read-only) ──────────────────

    #[actix_web::test]
    async fn test_umbra_query_account_unregistered() {
        tracing_subscriber::fmt::try_init().ok();

        let cmd = spawn_umbra_node("umbra_query_account").await;
        let ctx = test_utils::test_context();

        // Use a random keypair — guaranteed unregistered
        let kp = solana_keypair::Keypair::new();

        let output = cmd
            .run(
                ctx,
                value::map! {
                    "keypair" => kp.to_bytes(),
                    "network" => "mainnet",
                    "rpc_url" => "https://api.mainnet-beta.solana.com",
                },
            )
            .await
            .unwrap();

        let exists = value::from_value::<bool>(output["exists"].clone()).unwrap();
        assert!(!exists, "fresh keypair should not be registered");
    }

    #[actix_web::test]
    async fn test_umbra_query_balance_unregistered() {
        tracing_subscriber::fmt::try_init().ok();

        let cmd = spawn_umbra_node("umbra_query_balance").await;
        let ctx = test_utils::test_context();

        let kp = solana_keypair::Keypair::new();

        let output = cmd
            .run(
                ctx,
                value::map! {
                    "keypair" => kp.to_bytes(),
                    "network" => "mainnet",
                    "rpc_url" => "https://api.mainnet-beta.solana.com",
                    "mint" => "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                },
            )
            .await
            .unwrap();

        let balance = value::from_value::<String>(output["balance"].clone()).unwrap();
        assert_eq!(
            balance, "0",
            "unregistered account should have zero balance"
        );
    }

    #[actix_web::test]
    async fn test_umbra_fetch_utxos_devnet_returns_empty() {
        tracing_subscriber::fmt::try_init().ok();

        let cmd = spawn_umbra_node("umbra_fetch_utxos").await;
        let ctx = test_utils::test_context();

        let kp = solana_keypair::Keypair::new();

        let output = cmd
            .run(
                ctx,
                value::map! {
                    "keypair" => kp.to_bytes(),
                    "network" => "devnet",
                    "rpc_url" => "https://api.devnet.solana.com",
                },
            )
            .await
            .unwrap();

        let count = value::from_value::<f64>(output["count"].clone()).unwrap();
        assert_eq!(count, 0.0, "devnet has no indexer, should return 0 UTXOs");
    }

    // ── Umbra write tests ──────────────────────────────────────────────
    // NOTE: Umbra devnet program (342qFp...) has its programData closed,
    //       so write operations only work on mainnet.

    #[actix_web::test]
    #[ignore = "requires funded mainnet wallet: TEST_WALLET_KEYPAIR"]
    async fn test_umbra_register_devnet() {
        tracing_subscriber::fmt::try_init().ok();

        let cmd = spawn_umbra_node("umbra_register").await;
        let ctx = test_utils::test_context();

        let wallet = test_utils::test_wallet();
        let kp = wallet.keypair().unwrap();

        let output = cmd
            .run(
                ctx,
                value::map! {
                    "keypair" => kp.to_bytes(),
                    "network" => "devnet",
                    "rpc_url" => "https://api.devnet.solana.com",
                    "confidential" => true,
                    "anonymous" => false,
                },
            )
            .await
            .unwrap();

        let sig = value::from_value::<String>(output["signature"].clone()).unwrap();
        assert!(!sig.is_empty(), "expected a transaction signature");
        tracing::info!("register signature: {sig}");
    }

    #[actix_web::test]
    #[ignore = "requires TEST_WALLET_KEYPAIR"]
    async fn test_umbra_query_account_with_test_wallet() {
        tracing_subscriber::fmt::try_init().ok();

        let cmd = spawn_umbra_node("umbra_query_account").await;
        let ctx = test_utils::test_context();

        let wallet = test_utils::test_wallet();
        let kp = wallet.keypair().unwrap();

        let output = cmd
            .run(
                ctx,
                value::map! {
                    "keypair" => kp.to_bytes(),
                    "network" => "devnet",
                    "rpc_url" => "https://api.devnet.solana.com",
                },
            )
            .await
            .unwrap();

        let exists = value::from_value::<bool>(output["exists"].clone()).unwrap();
        tracing::info!(
            "test wallet {} registered on Umbra devnet: {exists}",
            kp.pubkey()
        );
    }
}
