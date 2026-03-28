# cmds-bun — Compiled Bun Nodes

Pre-built TypeScript nodes that run via [Bun](https://bun.sh) subprocesses. Same HTTP RPC protocol as `cmds-deno`, but uses `Bun.serve()` and gets its source embedded at compile time via `build.rs`.

## Architecture

```
┌─────────────────────┐       HTTP POST /call       ┌──────────────────────┐
│   Rust backend      │ ──────────────────────────▶ │   Bun subprocess     │
│                     │                             │                      │
│  BunCommand         │ ◀────────────────────────── │  bun-command-rpc     │
│  (RpcCommandClient) │       JSON response         │  (Bun.serve())       │
└─────────────────────┘                             └──────────────────────┘
        │                                                    │
        │ include_str!()                                     │ imports
        ▼                                                    ▼
  .jsonc + .ts pairs                              @space-operator/flow-lib-bun
  (node-definitions/)                             (Value, Context, BaseCommand)
```

### How It Works

1. **At `cargo build`** — `build.rs` scans `node-definitions/` for `.jsonc`+`.ts` pairs and generates `inventory::submit!()` registrations with scoped names like `@spo/{prefix}.{name}.{version}`.

2. **At runtime** — When a node is invoked, `new_owned()`:
   - Writes `cmd.ts`, `node-data.json`, `run.ts`, and `package.json` to a tempdir
   - Symlinks `node_modules/` from the nearest workspace root
   - Spawns `bun run run.ts`
   - Reads the port from stdout (first line)
   - Communicates via HTTP `POST /call` using `RpcCommandClient`

3. **On destroy** — The Bun subprocess is killed.

## Adding a New Node

Drop two files — no Rust code changes needed:

```
crates/cmds-bun/node-definitions/{category}/{name}.jsonc   ← ports, metadata
crates/cmds-bun/src/{category}/{name}.ts                   ← implementation
```

The paired `.ts` file is the runtime source of truth for compiled Bun nodes. The JSONC definition supplies typed ports, metadata, and the first-class `type: "bun"` registration.

### `.jsonc` — Node Definition

```jsonc
{
  "type": "bun",
  "name": "my_node",
  "prefix": "category",
  "version": "0.1",
  "author_handle": "spo",
  "source_code": "crates/cmds-bun/src/category/my_node.ts",
  "description": "Does something useful",
  "ports": {
    "inputs": [
      { "name": "input_a", "type_bounds": ["string"], "required": true, "passthrough": false }
    ],
    "outputs": [
      { "name": "result", "type": "string", "optional": false }
    ]
  },
  "config_schema": {},
  "config": {}
}
```

### `.ts` — Implementation

```typescript
import { BaseCommand, Context } from "@space-operator/flow-lib-bun";

export default class MyNode extends BaseCommand {
  override async run(_ctx: Context, inputs: any): Promise<any> {
    return { result: `Hello ${inputs.input_a}` };
  }
}

// ── Inline Tests ──────────────────────────────────────────────────────
import { test, expect, describe } from "bun:test";

describe("MyNode", () => {
  test("can be instantiated", () => {
    const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
    const cmd = new MyNode(nd);
    expect(cmd).toBeInstanceOf(BaseCommand);
  });
});
```

## Package Layout

| Package | Role |
|---------|------|
| `crates/cmds-bun/` | Rust crate — subprocess spawner + `build.rs` auto-discovery |
| `@space-operator/flow-lib-bun/` | `Value`, `Context`, `BaseCommand` — npm-compatible flow-lib |
| `@space-operator/bun-command-rpc/` | `Bun.serve()` RPC server handling `POST /call` |

### Dependency Graph

```
cmds-bun (Rust)
  └── embeds .jsonc + .ts via include_str!()

run.ts (Bun entrypoint)
  └── @space-operator/bun-command-rpc
        └── @space-operator/flow-lib-bun
              ├── @solana/web3.js
              ├── bs58
              ├── @stablelib/base64
              └── @msgpack/msgpack
```

## Workspace Setup

The root `package.json` defines the Bun workspace:

```json
{
  "name": "flow-backend",
  "private": true,
  "workspaces": [
    "@space-operator/bun-command-rpc",
    "@space-operator/flow-lib-bun",
    "crates/cmds-bun"
  ]
}
```

Install all workspace deps:

```bash
bun install
```

## Testing

All `.ts` node files include inline `bun:test` smoke tests. A wrapper file `umbra.test.ts` imports all nodes so Bun discovers them (Bun requires `*.test.ts` naming). There is also a Rust integration test that launches Bun end-to-end through the tempdir harness.

```bash
# Run everything
cargo test -p cmds-bun
bun test @space-operator/flow-lib-bun/src/ \
         @space-operator/bun-command-rpc/src/ \
         crates/cmds-bun/src/umbra/

# Individual suites
cargo test -p cmds-bun                          # Rust launcher + Bun subprocess smoke test
bun test @space-operator/flow-lib-bun/src/      # Value, BaseCommand, serialization
bun test @space-operator/bun-command-rpc/src/    # RPC server integration
bun test crates/cmds-bun/src/umbra/              # Umbra node smoke tests
```

## Existing Nodes

8 Umbra Privacy nodes wrapping `@umbra-privacy/sdk`:

| Node | Operation |
|------|-----------|
| `umbra_register` | Register user (confidential + anonymous modes) |
| `umbra_deposit` | Deposit tokens ATA → encrypted balance |
| `umbra_withdraw` | Withdraw tokens encrypted → public ATA |
| `umbra_query_account` | Query on-chain user account state |
| `umbra_query_balance` | Query & decrypt encrypted balance |
| `umbra_create_utxo` | Create receiver-claimable UTXO in mixer |
| `umbra_fetch_utxos` | Fetch claimable UTXOs from indexer |
| `umbra_claim_utxo` | Claim UTXO into encrypted balance |

Generated scoped names at build:
```
@spo/umbra.umbra_register.0.1
@spo/umbra.umbra_deposit.0.1
...
```

## Key Differences from cmds-deno

| | cmds-deno | cmds-bun |
|---|-----------|----------|
| **Runtime** | Deno subprocess | Bun subprocess |
| **Import style** | `npm:` / `jsr:` specifiers | Bare npm specifiers |
| **RPC server** | Oak (Deno) | `Bun.serve()` |
| **Flow lib** | `@space-operator/flow-lib` (JSR) | `@space-operator/flow-lib-bun` (npm) |
| **Node discovery** | Regex-based at runtime | `build.rs` `include_str!()` at compile time |
| **Source embedding** | `config.source` field | Auto-generated `inventory::submit!()` |
