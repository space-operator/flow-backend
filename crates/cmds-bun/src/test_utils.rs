//! Shared test utilities for integration testing of Bun commands.
//!
//! Mirrors `cmds-solana::test_utils` — provides wallet loading, devnet funding,
//! and a `CommandContext` wired to actually sign and submit transactions.
//!
//! # Environment Variables
//!
//! - `TEST_WALLET_KEYPAIR` — Base58-encoded keypair (or fall back to `keypair`)
//! - `SOLANA_DEVNET_URL` — Custom devnet RPC URL (optional)

use flow_lib::context::CommandContext;
use solana_keypair::Keypair;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

/// Load the test wallet from env var (base58 keypair).
///
/// Checks `TEST_WALLET_KEYPAIR` first, then falls back to `keypair`.
/// Panics if neither is set.
pub fn test_wallet() -> flow_lib::solana::Wallet {
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

        for _ in 0..30 {
            if client.confirm_transaction(&sig).await.unwrap_or(false) {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        panic!("airdrop confirmation timed out");
    }
}

/// Create a `CommandContext` with a real Solana execute service (devnet)
/// and the HTTP RPC server extension that Bun commands need.
///
/// Unlike `CommandContext::test_context()` which stubs out execution,
/// this context can actually sign and submit transactions on devnet.
pub fn test_context() -> CommandContext {
    let mut ctx = flow_lib_solana::utils::test_context_with_execute();
    ctx.extensions_mut()
        .unwrap()
        .insert(tower_rpc::Server::start_http_server().unwrap());
    ctx
}
