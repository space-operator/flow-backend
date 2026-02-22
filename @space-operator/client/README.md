# @space-operator/client

TypeScript SDK for the [Space Operator](https://spaceoperator.com) platform, built on [Effect](https://effect.website).

Every operation returns an `Effect<A, E, R>` where errors (`E`) are tracked in the type system and dependencies (`R`) are injected via Layers.

## Quick Start

```ts
import { Effect } from "effect";
import { FlowService, runFlow, SpaceOperatorFromEnv } from "@space-operator/client";

// Simplest: run a flow and get the output (polls until done)
const output = await Effect.runPromise(
  runFlow("my-flow-id", { inputs: { x: 42 } }).pipe(
    Effect.provide(SpaceOperatorFromEnv),
  ),
);

console.log(output);
```

`SpaceOperatorFromEnv` reads these environment variables:

| Variable | Required | Default |
|----------|----------|---------|
| `SPACE_OPERATOR_TOKEN` | Yes | |
| `SPACE_OPERATOR_HOST` | No | `https://dev-api.spaceoperator.com` |
| `SPACE_OPERATOR_ANON_KEY` | No | |

## Setup

### From Environment

```ts
import { Effect } from "effect";
import { FlowService, SpaceOperatorFromEnv } from "@space-operator/client";

const program = Effect.gen(function* () {
  const flow = yield* FlowService;
  return yield* flow.startFlow("my-flow", { inputs: {} });
}).pipe(Effect.provide(SpaceOperatorFromEnv));
```

### Manual Config

```ts
import { Effect } from "effect";
import { FlowService, SpaceOperatorLive, makeConfig } from "@space-operator/client";

const program = Effect.gen(function* () {
  const flow = yield* FlowService;
  return yield* flow.startFlow("my-flow", { inputs: {} });
}).pipe(
  Effect.provide(SpaceOperatorLive),
  Effect.provide(makeConfig({ token: "b3-my-api-key" })),
);
```

## Convenience Functions

These are standalone functions that compose lower-level service methods. Import them directly:

```ts
import { runFlow, runFlowWs } from "@space-operator/client";
```

### `runFlow(id, params, opts?)` -- Run a Flow (HTTP Polling)

Starts a flow and polls `getFlowOutput` with exponential backoff until the result is available. No WebSocket required.

```ts
import { Effect } from "effect";
import { runFlow, SpaceOperatorFromEnv } from "@space-operator/client";

const output = await Effect.runPromise(
  runFlow("flow-id", {
    inputs: { name: "world" },
  }).pipe(Effect.provide(SpaceOperatorFromEnv)),
);
```

Options:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `baseDelay` | `number` | `1000` | Initial polling interval (ms) |
| `maxDelay` | `number` | `5000` | Max polling interval cap (ms) |
| `timeout` | `number` | `300000` | Total timeout (ms) |

Requires: `FlowService`

### `runFlowWs(id, params, opts?)` -- Run a Flow (WebSocket)

Starts a flow, subscribes to events via WebSocket, waits for `FlowFinish`, then fetches the output. Connects and authenticates the WS automatically if needed.

```ts
import { Effect } from "effect";
import { runFlowWs, SpaceOperatorFromEnv } from "@space-operator/client";

const { output, events } = await Effect.runPromise(
  runFlowWs(
    "flow-id",
    { inputs: {} },
    {
      onEvent: (ev) => console.log(`[${ev.event}]`, ev.data),
      collectEvents: true,
    },
  ).pipe(Effect.provide(SpaceOperatorFromEnv)),
);
```

Options:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `onEvent` | `(event) => void` | | Called for each event as it arrives |
| `collectEvents` | `boolean` | `false` | Return all events in the result |
| `timeout` | `number` | `300000` | Total timeout (ms) |

Returns: `{ output: Value, events: FlowRunEvent[] }`

Requires: `FlowService | WsService`

## Services

All services are accessed via Effect's `Context.Tag` pattern:

```ts
const program = Effect.gen(function* () {
  const svc = yield* SomeService;
  return yield* svc.method(args);
});
```

### FlowService

Manage flow execution, deployments, and Solana transaction signing.

```ts
import { Effect } from "effect";
import { FlowService, SpaceOperatorFromEnv } from "@space-operator/client";

const program = Effect.gen(function* () {
  const flow = yield* FlowService;

  // Start a flow
  const { flow_run_id } = yield* flow.startFlow("flow-id", {
    inputs: { key: { type: "string", value: "hello" } },
  });

  // Get output (single attempt -- use runFlow for polling)
  const output = yield* flow.getFlowOutput(flow_run_id);

  return output;
}).pipe(Effect.provide(SpaceOperatorFromEnv));
```

#### Methods

| Method | Parameters | Returns | Description |
|--------|-----------|---------|-------------|
| `startFlow` | `(id, params)` | `{ flow_run_id }` | Start an authenticated flow |
| `startFlowShared` | `(id, params)` | `{ flow_run_id }` | Start a shared flow |
| `startFlowUnverified` | `(id, publicKey, params)` | `{ flow_run_id, token }` | Start without auth (uses pubkey as identity) |
| `stopFlow` | `(runId, params)` | `{ success }` | Stop a running flow |
| `getFlowOutput` | `(runId, token?)` | `Value` | Fetch output (fails if not finished) |
| `getSignatureRequest` | `(runId, token?)` | `SignatureRequest` | Get pending signature request |
| `submitSignature` | `(params)` | `{ success }` | Submit a raw signature |
| `signAndSubmitSignature` | `(req, publicKey, signFn)` | `void` | Sign a transaction and submit |
| `deployFlow` | `(id)` | `string` | Create a deployment, returns deployment_id |
| `startDeployment` | `(spec, params?, token?)` | `{ flow_run_id, token }` | Start a deployed flow |
| `exportData` | `()` | `Record<string, unknown>` | Export all user data |
| `importData` | `(data)` | `void` | Import data |

#### Signing Transactions

When a flow requires a Solana signature:

```ts
const program = Effect.gen(function* () {
  const flow = yield* FlowService;

  const { flow_run_id } = yield* flow.startFlow("my-flow", { inputs: {} });

  // Get the signature request (poll or use WS events to know when it's ready)
  const req = yield* flow.getSignatureRequest(flow_run_id);

  // Sign and submit (handles transaction building, signing, and submission)
  yield* flow.signAndSubmitSignature(
    req,
    walletPublicKey,
    (tx) => wallet.signTransaction(tx),
  );

  const output = yield* flow.getFlowOutput(flow_run_id);
  return output;
});
```

#### Deployments

```ts
const program = Effect.gen(function* () {
  const flow = yield* FlowService;

  // Deploy a flow (creates a public endpoint)
  const deploymentId = yield* flow.deployFlow("flow-id");

  // Start the deployment (can be called by anyone with the deployment ID)
  const { flow_run_id, token } = yield* flow.startDeployment(
    { id: deploymentId },
    { inputs: {} },
  );

  // Use the token to fetch output (no auth needed)
  const output = yield* flow.getFlowOutput(flow_run_id, token);
  return output;
});
```

#### Unverified Flows

For flows that authenticate via a Solana public key instead of an API token:

```ts
const program = Effect.gen(function* () {
  const flow = yield* FlowService;

  const { flow_run_id, token } = yield* flow.startFlowUnverified(
    "flow-id",
    "SoLaNaPuBkEy...",
    { inputs: {} },
  );

  // Use the returned token for subsequent requests
  const output = yield* flow.getFlowOutput(flow_run_id, token);
  return output;
});
```

### AuthService

Solana wallet authentication and token management.

```ts
import { Effect } from "effect";
import { AuthService, SpaceOperatorFromEnv } from "@space-operator/client";

// Solana wallet login
const program = Effect.gen(function* () {
  const auth = yield* AuthService;

  // Step 1: Get message to sign
  const msg = yield* auth.initAuth("SoLaNaPuBkEyBaSe58...");

  // Step 2: Sign the message with your wallet (app-specific)
  const signature = signMessage(msg); // returns Uint8Array or base58 string

  // Step 3: Submit signature, get Supabase session
  const { session, new_user } = yield* auth.confirmAuth(msg, signature);

  return session;
}).pipe(Effect.provide(SpaceOperatorFromEnv));
```

#### Methods

| Method | Parameters | Returns | Description |
|--------|-----------|---------|-------------|
| `initAuth` | `(pubkey)` | `string` | Get the message to sign |
| `confirmAuth` | `(msg, signature)` | `{ session, new_user }` | Submit signature, get session |
| `claimToken` | `()` | `{ access_token, refresh_token }` | Exchange API key for short-lived JWT |

### WsService

WebSocket connection with automatic reconnection and Stream-based subscriptions.

```ts
import { Effect, Stream } from "effect";
import { WsService, FlowService, SpaceOperatorFromEnv } from "@space-operator/client";

const program = Effect.gen(function* () {
  const ws = yield* WsService;
  const flow = yield* FlowService;

  // Connect and authenticate
  yield* ws.connect();
  yield* ws.authenticate();

  // Start a flow
  const { flow_run_id } = yield* flow.startFlow("my-flow", { inputs: {} });

  // Subscribe to events (returns a Stream)
  const events = ws.subscribeFlowRunEvents(flow_run_id);

  // Process events
  yield* events.pipe(
    Stream.tap((ev) =>
      Effect.log(`[${ev.event}] ${JSON.stringify(ev.data)}`),
    ),
    Stream.runDrain,
  );

  // Stream ends automatically after FlowFinish
  yield* ws.close();
}).pipe(Effect.provide(SpaceOperatorFromEnv));
```

#### Methods

| Method | Parameters | Returns | Description |
|--------|-----------|---------|-------------|
| `connect` | `()` | `void` | Open WebSocket connection |
| `authenticate` | `()` | `AuthenticateResponseOk` | Authenticate over the connection |
| `subscribeFlowRunEvents` | `(flowRunId, token?)` | `Stream<FlowRunEvent>` | Subscribe to flow events |
| `subscribeSignatureRequests` | `()` | `Stream<SignatureRequestsEvent>` | Subscribe to signature requests |
| `close` | `()` | `void` | Close the connection |
| `state` | | `WsConnectionState` | Current connection state |

#### Event Types

The `FlowRunEvent` stream emits these events:

| Event | Description |
|-------|-------------|
| `FlowStart` | Flow execution started |
| `FlowFinish` | Flow execution completed (stream ends) |
| `FlowError` | Flow-level error occurred |
| `FlowLog` | Flow-level log message |
| `NodeStart` | A node started executing |
| `NodeOutput` | A node produced output |
| `NodeFinish` | A node finished |
| `NodeError` | A node encountered an error |
| `NodeLog` | A node produced a log message |
| `SignatureRequest` | A signature is needed |
| `ApiInput` | External API input requested |

#### Reconnection

The WS service automatically reconnects on unexpected disconnects:
- Exponential backoff: 1s, 2s, 4s, ... capped at 30s (with jitter)
- Max 10 retries by default
- Active subscriptions are re-established after reconnect
- Stream consumers see no interruption (events resume from where they left off)

### KvService

Key-value store operations.

```ts
import { Effect } from "effect";
import { KvService, SpaceOperatorFromEnv } from "@space-operator/client";

const program = Effect.gen(function* () {
  const kv = yield* KvService;

  // Create a store
  yield* kv.createStore("my-store");

  // Write a value (IValue format)
  yield* kv.writeItem("my-store", "greeting", {
    type: "string",
    value: "hello world",
  });

  // Read it back
  const { value } = yield* kv.readItem("my-store", "greeting");
  console.log(value); // { type: "string", value: "hello world" }

  // Delete the item
  yield* kv.deleteItem("my-store", "greeting");

  // Delete the store
  yield* kv.deleteStore("my-store");
}).pipe(Effect.provide(SpaceOperatorFromEnv));
```

#### Methods

| Method | Parameters | Returns | Description |
|--------|-----------|---------|-------------|
| `createStore` | `(store)` | `void` | Create a new KV store |
| `deleteStore` | `(store)` | `void` | Delete a KV store |
| `writeItem` | `(store, key, value)` | `{ old_value }` | Write a value (returns previous value) |
| `readItem` | `(store, key)` | `{ value }` | Read a value |
| `deleteItem` | `(store, key)` | `{ old_value }` | Delete a value (returns deleted value) |

### ApiKeyService

Manage API keys and query server info.

```ts
import { Effect } from "effect";
import { ApiKeyService, SpaceOperatorFromEnv } from "@space-operator/client";

const program = Effect.gen(function* () {
  const apiKey = yield* ApiKeyService;

  // Create a new API key
  const { full_key, key_hash } = yield* apiKey.create("my-key");
  console.log("New key:", full_key); // b3-...

  // Get info about the current key
  const { user_id } = yield* apiKey.info();

  // Get server info (public, no auth needed)
  const info = yield* apiKey.serverInfo();
  console.log("Server:", info.base_url);

  // Delete a key by its hash
  yield* apiKey.delete(key_hash);
}).pipe(Effect.provide(SpaceOperatorFromEnv));
```

#### Methods

| Method | Parameters | Returns | Description |
|--------|-----------|---------|-------------|
| `create` | `(name)` | `{ full_key, key_hash, trimmed_key, name, user_id, created_at }` | Create API key |
| `delete` | `(keyHash)` | `void` | Delete API key |
| `info` | `()` | `{ user_id }` | Get current key info |
| `serverInfo` | `()` | `{ supabase_url, anon_key, iroh, base_url }` | Server info (public) |

### WalletService

Manage wallet entries.

```ts
import { Effect } from "effect";
import { WalletService, SpaceOperatorFromEnv } from "@space-operator/client";

const program = Effect.gen(function* () {
  const wallet = yield* WalletService;

  const result = yield* wallet.upsertWallet({
    public_key: "SoLaNaPuBkEy...",
    type: "HARDCODED",
    name: "my-wallet",
    keypair: "base58-encoded-keypair",
  });

  console.log("Wallet ID:", result[0].id);
}).pipe(Effect.provide(SpaceOperatorFromEnv));
```

#### Methods

| Method | Parameters | Returns | Description |
|--------|-----------|---------|-------------|
| `upsertWallet` | `(body)` | `[{ id, public_key }]` | Create or update a wallet |

## Error Handling

All errors are `Data.TaggedError` instances with a `_tag` field. Use `Effect.catchTag` to handle specific errors:

```ts
import { Effect } from "effect";
import { runFlow, HttpApiError, AuthTokenError, SpaceOperatorFromEnv } from "@space-operator/client";

const program = runFlow("flow-id", { inputs: {} }).pipe(
  Effect.catchTag("HttpApiError", (err) =>
    Effect.gen(function* () {
      console.error(`HTTP ${err.status}: ${err.message}`);
      return yield* Effect.fail(err);
    }),
  ),
  Effect.catchTag("AuthTokenError", (err) =>
    Effect.gen(function* () {
      console.error("Auth failed:", err.message);
      return yield* Effect.fail(err);
    }),
  ),
  Effect.provide(SpaceOperatorFromEnv),
);
```

### Error Types

| Error | Fields | When |
|-------|--------|------|
| `HttpApiError` | `status`, `url`, `body`, `message` | HTTP request failed or returned non-2xx |
| `AuthTokenError` | `message` | No token configured or token invalid |
| `WsProtocolError` | `method`, `message` | WS server returned an error response |
| `WsConnectionError` | `message` | WS connection failed or dropped |
| `WsTimeoutError` | `message` | WS request or connection timed out |

## Layer Architecture

```
SpaceOperatorFromEnv
  = SpaceOperatorLive + SpaceOperatorConfigFromEnv

SpaceOperatorLive (requires SpaceOperatorConfig)
  ├── AuthServiceLive ─────┐
  ├── FlowServiceLive ─────┤
  ├── KvServiceLive ───────┼── all depend on SpaceHttpClientLive
  ├── ApiKeyServiceLive ───┤
  ├── WalletServiceLive ───┘
  └── WsServiceLive ─── depends on SpaceOperatorConfig directly

SpaceHttpClientLive
  └── depends on SpaceOperatorConfig + FetchHttpClient
```

Use `SpaceOperatorFromEnv` for the common case (reads env vars). Use `SpaceOperatorLive` + `makeConfig()` for programmatic configuration.
