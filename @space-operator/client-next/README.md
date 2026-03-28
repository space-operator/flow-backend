# `@space-operator/client-next`

The in-progress rewrite of the Space Operator TypeScript client.

This package lives in `@space-operator/client-next` while the rewrite is under
development. The intent is to promote it into `@space-operator/client` once the
new API is validated.

## TODO

- Write a migration guide from the legacy `Client` / `WsClient` API to
  `createClient(...)`, capability namespaces, `FlowRunHandle`, and
  `client.ws()`.
- Add CI regression coverage so unit/runtime tests run automatically and the
  live E2E suite can run in a gated environment when the required services and
  secrets are available.

## Goals

- Explicit auth strategies instead of mutable `setToken()` state.
- Namespace-based API surface instead of a single large client class.
- Typed HTTP and websocket transport with shared error handling.
- Shared Zod contracts and generated OpenAPI types for backend-facing payloads.
- Built-in telemetry hooks for HTTP and websocket execution paths.
- First-class run handles for flow and deployment execution.
- Cross-runtime TypeScript support for Deno, browser-like runtimes, and
  Node-like runtimes with injected `fetch` and `WebSocket`.

## Highlights

- `createClient({ baseUrl, auth?, anonKey?, fetch?, webSocketFactory?, logger?, retry?, timeoutMs? })`
- Explicit auth helpers:
  - `apiKeyAuth(...)`
  - `bearerAuth(...)`
  - `flowRunTokenAuth(...)`
  - `publicKeyAuth(...)`
- Capability namespaces:
  - `auth`
  - `flows`
  - `deployments`
  - `events`
  - `signatures`
  - `wallets`
  - `apiKeys`
  - `kv`
  - `data`
  - `service`
- Higher-level ergonomics:
  - `FlowRunHandle`
  - `WebSocketSession`
  - `signAndSubmitSignature(...)`
- Internal platform pieces:
  - shared schemas from `@space-operator/contracts`
  - OpenTelemetry spans for HTTP and websocket operations
  - generated server OpenAPI types
- Subpath exports:
  - `@space-operator/client-next/solana`
  - `@space-operator/client-next/x402`

## Quick Start

```ts
import { apiKeyAuth, createClient } from "@space-operator/client-next";

const client = createClient({
  baseUrl: "http://localhost:8080",
  auth: apiKeyAuth(Deno.env.get("APIKEY")!),
});

const run = await client.flows.start("6c949718-69e2-47c1-8b93-d56b8e34ec51", {
  inputs: {
    a: 1,
    b: 2,
  },
});

const output = await run.output();
console.log(output.toJSObject());
```

## Auth

The rewrite does not infer auth mode from token shape. Auth is always explicit.

```ts
import {
  apiKeyAuth,
  bearerAuth,
  createClient,
  publicKeyAuth,
} from "@space-operator/client-next";

const owner = createClient({
  baseUrl: "http://localhost:8080",
  auth: apiKeyAuth(Deno.env.get("APIKEY")!),
});

const anonymous = createClient({
  baseUrl: "http://localhost:8080",
});

const publicKeyClient = createClient({
  baseUrl: "http://localhost:8080",
  auth: publicKeyAuth("YourPublicKeyBase58"),
});

const bearerClient = owner.withAuth(
  bearerAuth("supabase-access-token"),
);
```

### Wallet Login

```ts
import { createClient, web3 } from "@space-operator/client-next";
import * as nacl from "tweetnacl";

const keypair = web3.Keypair.generate();
const client = createClient({
  baseUrl: "http://localhost:8080",
});

const session = await client.auth.loginWithSignature(
  keypair.publicKey,
  (message) =>
    nacl.sign.detached(new TextEncoder().encode(message), keypair.secretKey),
);

console.log(session.session.access_token);
```

If you do not pass `anonKey`, auth bootstrap will discover it from
`client.service.info()`.

## Flows

### Start A Flow

```ts
const run = await client.flows.start(flowId, {
  inputs: {
    a: 1,
    b: 2,
  },
});

const output = await run.output();
```

### Shared And Unverified Starts

```ts
const sharedRun = await client.flows.startShared(flowId, {
  inputs: { a: 1, b: 2 },
});

const unverifiedRun = await client.flows.startUnverified(flowId, {
  inputs: { a: 1, b: 2 },
});
```

### Clone, Stop, And Wait

```ts
const cloned = await client.flows.clone(flowId);
const run = await client.flows.start(cloned.flow_id, {
  inputs: { a: 4, b: 5 },
});

await run.waitForFinish();
await run.stop({ reason: "cleanup" }).catch(() => undefined);
```

## Deployments

Deployment starts return the same `FlowRunHandle` abstraction as direct flow
starts.

```ts
const run = await client.deployments.start(
  { flow: flowId, tag: "latest" },
  {
    inputs: {
      sender: "SomePubkey",
      n: 2,
    },
  },
);

const output = await run.output();
```

You can also start by deployment id:

```ts
const run = await client.deployments.start(
  { id: deploymentId },
  { inputs: { n: 2 } },
);
```

## Run Handles

`FlowRunHandle` carries the run id and optional flow-run token returned by
unverified or deployment starts.

```ts
const run = await client.flows.startUnverified(flowId, {
  inputs: { a: 7, b: 8 },
});

console.log(run.id);
console.log(run.token);

const request = await run.signatureRequest();
const finish = await run.waitForFinish();
```

Available helpers:

- `run.output()`
- `run.stop()`
- `run.signatureRequest()`
- `run.events()`
- `run.waitForFinish()`
- `run.withAuth(...)`

## Realtime

The client supports both one-shot subscriptions and reusable websocket sessions.

### One-Shot Subscription

```ts
const subscription = await client.events.subscribeFlowRun(run.id);

for await (const event of subscription) {
  console.log(event.event, event.data);
}
```

### Reusable Session

```ts
const ws = client.ws();
await ws.authenticate();

const flowEvents = await ws.subscribeFlowRun(run.id);
const signatureEvents = await ws.subscribeSignatureRequests();

// consume both streams

await flowEvents.close();
await signatureEvents.close();
await ws.close();
```

## Signature Requests

You can submit signatures directly:

```ts
await client.signatures.submit({
  id: request.id,
  signature: "base58-signature",
});
```

Or use the Solana helper:

```ts
import {
  signAndSubmitSignature,
  web3,
} from "@space-operator/client-next/solana";

await signAndSubmitSignature(client.signatures, request, {
  publicKey: keypair.publicKey,
  signTransaction: async (tx) => {
    tx.sign([keypair]);
    return tx;
  },
});
```

## Other Namespaces

```ts
import { bearerAuth } from "@space-operator/client-next";

const owner = client.withAuth(bearerAuth(accessToken));

await client.service.info();
await client.service.healthcheck();

await owner.apiKeys.create("ci-key");

await owner.kv.createStore("store_name");
await owner.kv.write("store_name", "key", { hello: "world" });
await owner.kv.read("store_name", "key");

await owner.wallets.upsert({
  type: "HARDCODED",
  name: "wallet-name",
  public_key: "WalletPubkey",
  user_id: "user-id",
});

await owner.data.export();
```

## Errors

The transport layer throws typed errors instead of raw string failures:

- `ApiError`
- `TransportError`
- `TimeoutError`
- `AbortError`
- `WebSocketProtocolError`
- `FlowRunFailedError`

## Runtime Notes

This package is designed to work across multiple runtimes:

- Deno
- Browser-like environments with global `fetch` and `WebSocket`
- Node-like environments when you inject `fetch` and a websocket factory

Relevant options:

- `fetch`
- `webSocketFactory`
- `retry`
- `timeoutMs`
- `logger`
- `telemetry`

The package typechecks on the current repo runtime. This repo now targets Deno
`2.6.4` in local development, CI, and Docker, and the browser-bundling
Playwright smoke test assumes a Deno 2 toolchain.

## Testing

The package includes unit, runtime, and live E2E coverage.

- Test guide:
  [tests/README.md](/home/amir/code/space-operator/flow-backend/@space-operator/client-next/tests/README.md)
- Unit and runtime tests validate transport and runtime behavior.
- `tests/playwright` contains the browser websocket smoke coverage.
- `tests/contract` is the live E2E suite for the full backend surface.

## Status

This is the rewrite branch of the client, not the final published cutover yet.

- The old client remains in `@space-operator/client`.
- The new API is namespace-based and intentionally not source-compatible with
  the old `Client` / `WsClient` class pair.
- The staged rewrite currently has unit, runtime, and end-to-end coverage for
  the exported namespaces in this folder.
