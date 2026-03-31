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
    path::{Path, PathBuf},
    process::Stdio,
};
use tempfile::tempdir;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::{Child, Command},
};
use url::Url;

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

/// Known companion modules that may be imported by cmd.ts via relative paths.
/// Maps the import specifier to the embedded source.
const COMPANION_MODULES: &[(&str, &str)] = &[(
    "./umbra_common.ts",
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/umbra/umbra_common.ts"
    )),
)];

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

pub(crate) async fn new_owned(nd: NodeData) -> Result<Box<dyn CommandTrait>, CommandError> {
    let source = source_from_config(&nd)
        .ok_or_else(|| CommandError::msg("bun command source/code not found"))?;

    let dir = tempdir()?;

    tokio::fs::write(dir.path().join("cmd.ts"), &source)
        .await
        .context("write cmd.ts")?;

    // Write well-known companion modules that cmd.ts may import via relative paths.
    write_companion_modules(dir.path(), &source).await?;

    let mut node_data = nd.clone();
    if let Some(obj) = node_data.config.as_object_mut() {
        obj.remove("code");
        obj.remove("source");
    }
    let node_data_json = serde_json::to_string(&node_data).context("serialize NodeData")?;
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

    let mut spawned = Command::new("bun")
        .current_dir(dir.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("NO_COLOR", "1")
        .kill_on_drop(true)
        .arg("run")
        .arg("run.ts")
        .spawn()
        .context("spawn bun")?;

    let mut stdout = BufReader::new(spawned.stdout.take().unwrap()).lines();
    let port = match stdout.next_line().await? {
        Some(line) => line.parse::<u16>().context("parse port")?,
        None => {
            let mut error = String::new();
            let mut stderr = BufReader::new(spawned.stderr.take().unwrap());
            stderr.read_to_string(&mut error).await.map_err(|error| {
                tracing::warn!("read error: {}", error);
                CommandError::msg("could not start bun command")
            })?;
            return Err(CommandError::msg(error));
        }
    };
    let base_url = Url::parse(&format!("http://127.0.0.1:{port}")).unwrap();
    let cmd = RpcCommandClient::new(base_url, String::new(), node_data.clone());
    let cmd = BunCommand {
        inner: cmd,
        spawned,
        source,
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
    inner: RpcCommandClient,
    spawned: Child,
    source: String,
}

#[async_trait(?Send)]
impl CommandTrait for BunCommand {
    fn r#type(&self) -> flow_lib::CommandType {
        flow_lib::CommandType::Bun
    }
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
