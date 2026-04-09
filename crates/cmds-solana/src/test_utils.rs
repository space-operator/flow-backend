//! Shared test utilities for integration testing of Solana commands.
//!
//! Provides helpers to load wallets from `.env`, ensure devnet funding,
//! and create a `CommandContext` wired to actually sign and submit transactions.
//!
//! # Environment Variables
//!
//! - `TEST_WALLET_KEYPAIR` - Base58-encoded keypair for test wallet (required for integration tests)
//! - `SOLANA_DEVNET_URL` - Custom devnet RPC URL (optional, defaults to public devnet)

use flow_lib::config::Endpoints;
use flow_lib::flow_run_events::NodeLogSender;
use flow_lib::{
    ContextConfig, FlowRunId, NodeId, SolanaNet, ValueSet,
    context::{
        CommandContext, CommandContextData, FlowContextData, FlowServices, FlowSetContextData,
        FlowSetServices, User,
        execute::{self, Error as ExecuteError},
        signer,
    },
    flow_run_events,
    solana::{ExecutionConfig, Instructions, Wallet},
    utils::tower_client::{CommonErrorExt, TowerClient, unimplemented_svc},
};
use flow_lib_solana::InstructionsExt;
use solana_keypair::Keypair;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use std::collections::HashMap;
use std::sync::Arc;
use tower::service_fn;

/// Load the test wallet from env var (base58 keypair).
///
/// Checks `TEST_WALLET_KEYPAIR` first, then falls back to `keypair`.
/// Panics if neither is set.
pub fn test_wallet() -> Wallet {
    dotenvy::dotenv().ok();
    let key = std::env::var("TEST_WALLET_KEYPAIR")
        .or_else(|_| std::env::var("keypair"))
        .expect("TEST_WALLET_KEYPAIR or keypair env var required for integration tests");
    Keypair::from_base58_string(&key).into()
}

/// Ensure the given pubkey has at least `min_sol` SOL on devnet.
/// Requests an airdrop if the balance is insufficient.
pub async fn ensure_funded(client: &RpcClient, pubkey: &solana_pubkey::Pubkey, min_sol: f64) {
    let balance = client.get_balance(pubkey).await.unwrap_or(0) as f64 / 1_000_000_000.0;

    if balance < min_sol {
        let lamports = (min_sol * 1_000_000_000.0) as u64;
        let sig = client
            .request_airdrop(pubkey, lamports.min(2_000_000_000))
            .await
            .expect("airdrop request failed");

        // Wait for airdrop confirmation
        for _ in 0..30 {
            if client.confirm_transaction(&sig).await.unwrap_or(false) {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        panic!("airdrop confirmation timed out");
    }
}

/// Create a `CommandContext` with a real execute service wired to devnet.
///
/// Unlike `CommandContext::default()` which stubs out execution,
/// this context can actually sign and submit transactions on devnet.
pub fn test_context() -> CommandContext {
    dotenvy::dotenv().ok();

    let config = ContextConfig::default();
    let solana_client = Arc::new(config.solana_client.build_client(None));
    let node_id = NodeId::nil();
    let times = 0;
    let (tx, _) = flow_run_events::channel();

    let rpc = solana_client.clone();
    let network = SolanaNet::Devnet;

    // Real execute service that signs with local keypairs and submits to devnet
    let execute_svc: execute::Svc = TowerClient::new(service_fn(move |req: execute::Request| {
        let rpc = rpc.clone();
        async move {
            let instructions = req.instructions;

            if instructions.instructions.is_empty() {
                return Ok(execute::Response { signature: None });
            }

            let config = ExecutionConfig::default();
            let signer = build_keypair_signer(&instructions);

            let signature = instructions
                .execute(&rpc, None, network, signer, None, config)
                .await
                .map_err(|e| ExecuteError::msg(e.to_string()))?;

            Ok(execute::Response {
                signature: Some(signature),
            })
        }
    }));

    CommandContext::builder()
        .data(CommandContextData {
            node_id,
            times,
            flow: FlowContextData {
                flow_run_id: FlowRunId::nil(),
                environment: HashMap::new(),
                inputs: ValueSet::default(),
                read_only: false,
                set: FlowSetContextData {
                    flow_owner: User::default(),
                    started_by: User::default(),
                    endpoints: Endpoints::default(),
                    solana: config.solana_client,
                    http: config.http_client,
                },
            },
        })
        .execute(execute_svc)
        .get_jwt(unimplemented_svc())
        .node_log(NodeLogSender::new(tx, node_id, times))
        .flow(
            FlowServices::builder()
                .signer(unimplemented_svc())
                .set(
                    FlowSetServices::builder()
                        .http(reqwest::Client::new())
                        .solana_client(solana_client)
                        .extensions(Default::default())
                        .api_input(unimplemented_svc())
                        .build(),
                )
                .build(),
        )
        .build()
}

/// Build a signer service from the keypairs in an Instructions set.
fn build_keypair_signer(instructions: &Instructions) -> signer::Svc {
    let keypairs: Vec<(solana_pubkey::Pubkey, String)> = instructions
        .signers
        .iter()
        .filter_map(|w| w.keypair().map(|kp| (kp.pubkey(), kp.to_base58_string())))
        .collect();

    TowerClient::new(service_fn(move |req: signer::SignatureRequest| {
        let keypairs = keypairs.clone();
        async move {
            let b58 = keypairs
                .iter()
                .find(|(pk, _)| *pk == req.pubkey)
                .map(|(_, b58)| b58.clone())
                .ok_or_else(|| signer::Error::Pubkey(req.pubkey.to_string()))?;

            let keypair = Keypair::from_base58_string(&b58);
            let signature = keypair.sign_message(&req.message);

            Ok(signer::SignatureResponse {
                signature,
                new_message: None,
            })
        }
    }))
}
