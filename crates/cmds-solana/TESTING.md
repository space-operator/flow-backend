# Integration Testing for cmds-solana

## Setup

1. Add your devnet keypair (base58) to `.env` at the repo root:
   ```
   keypair=<your-base58-keypair>
   ```

2. Ensure the wallet has devnet SOL. The `ensure_funded()` helper will auto-airdrop if needed.

## Writing an Integration Test

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;

    #[tokio::test]
    #[ignore = "requires funded wallet and network access"]
    async fn test_my_command() {
        tracing_subscriber::fmt::try_init().ok();

        let wallet = test_utils::test_wallet();
        let ctx = test_utils::test_context();

        // Airdrop if balance < 0.1 SOL
        test_utils::ensure_funded(ctx.solana_client(), &wallet.pubkey(), 0.1).await;

        let output = run(ctx, Input {
            fee_payer: wallet.clone(),
            // ... other fields
            submit: true,
        })
        .await
        .unwrap();

        assert!(output.signature.is_some());
    }
}
```

## Helpers (from `test_utils`)

| Helper | Description |
|---|---|
| `test_wallet()` | Loads keypair from `TEST_WALLET_KEYPAIR` or `keypair` env var |
| `test_context()` | `CommandContext` with a real execute service wired to devnet |
| `ensure_funded(client, pubkey, min_sol)` | Checks balance, airdrops if below threshold |

## Running Tests

```bash
# Run a specific integration test
cargo test -p cmds-solana system_program::transfer_sol::tests::test_transfer_sol -- --ignored --nocapture

# Run all integration tests
cargo test -p cmds-solana -- --ignored --nocapture

# Run only fast unit tests (no network)
cargo test -p cmds-solana
```

## How It Works

`test_context()` builds a `CommandContext` identical to `CommandContext::default()` but replaces the stubbed `execute` service with a real one that:

1. Extracts keypairs from the `Instructions` signers
2. Builds a `signer::Svc` that can sign with those keypairs
3. Calls `InstructionsExt::execute()` from `flow-lib-solana` to sign and submit to devnet

This means `ctx.execute(instructions, output)` inside `run()` actually submits the transaction on-chain.
