# Testing for cmds-bun

## Prerequisites

1. **Bun** installed (`bun --version`)
2. Run `bun install` at the repo root to populate `node_modules`
3. For Rust tests: standard `cargo` toolchain

## Test Suites

### 1. Bun Unit Tests (fast, no network)

Inline tests in each `.ts` file — instantiation and input validation.

```bash
bun test crates/cmds-bun/src/umbra/umbra.test.ts
```

### 2. Bun Integration Tests (mainnet RPC, read-only)

Exercises the node classes directly against mainnet Umbra. Read-only operations
(query_account, query_balance, fetch_utxos) use a throwaway keypair — no funds needed.

```bash
bun test crates/cmds-bun/src/umbra/umbra_integration.test.ts
```

Write operations (register, deposit, withdraw) are **skipped** unless you provide
a funded mainnet wallet:

```bash
UMBRA_TEST_KEYPAIR=<base58-secret-key> bun test crates/cmds-bun/src/umbra/umbra_integration.test.ts
```

The wallet needs mainnet SOL (for tx fees) and USDC for deposit/withdraw tests.

### 3. Rust Tests (Bun RPC pipeline)

Tests the full Rust -> Bun subprocess -> HTTP RPC -> TypeScript -> SDK pipeline.

```bash
# All tests (includes add.ts + umbra nodes)
cargo test -p cmds-bun -- --nocapture

# Just the umbra tests
cargo test -p cmds-bun umbra -- --nocapture
```

### 4. Run Everything

```bash
bun install
bun test crates/cmds-bun/src/umbra/
cargo test -p cmds-bun -- --nocapture
```

## Environment Variables

| Variable | Required | Description |
|---|---|---|
| `UMBRA_TEST_KEYPAIR` | For write tests | Base58-encoded Solana secret key (mainnet, funded) |
| `keypair` | For Solana tests | Base58-encoded keypair (devnet), used by `test_utils::test_wallet()` |

## Network Constraints

The Umbra Privacy program is **only deployed on mainnet**
(`C6KsXC5aFhffHVnaHn6LxQzM3SJGmdW6mB6FWNbwJ2Kr`). Devnet has a program ID
configured in the SDK but it is not deployed on-chain. All nodes validate the
network input and reject `localnet`.

The Umbra indexer (for `fetch_utxos`) and relayer (for `claim_utxo`) are
external services that may not always be reachable. If they are down, those
nodes will return clear error messages.

## Writing a New Bun Node Test

Follow the pattern in `src/lib.rs`:

```rust
#[actix_web::test]
async fn test_my_node() {
    tracing_subscriber::fmt::try_init().ok();

    let cmd = spawn_umbra_node("umbra_my_node").await;
    let ctx = test_utils::test_context();

    let output = cmd
        .run(ctx, value::map! {
            "keypair" => solana_keypair::Keypair::new().to_bytes(),
            "network" => "mainnet",
            "rpc_url" => "https://api.mainnet-beta.solana.com",
            // ... other inputs
        })
        .await
        .unwrap();

    // assert on output fields
}
```

For TypeScript-side tests, add cases to `umbra_integration.test.ts`.
