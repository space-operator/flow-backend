# Migrating From `@space-operator/client`

This guide maps the legacy `Client` / `WsClient` API to
`@space-operator/client-next`.

## High-Level Changes

- `Client` is replaced by `createClient(...)`.
- Mutable client state is gone. There is no `setToken()`, `setFetch()`, or
  `setLogger()` after construction.
- The new client is split into capability namespaces such as `auth`, `flows`,
  `deployments`, `events`, `signatures`, `wallets`, `apiKeys`, `kv`, `data`, and
  `service`.
- Flow and deployment starts return `FlowRunHandle`, which carries the run id,
  optional flow-run token, and helpers like `output()`, `stop()`,
  `signatureRequest()`, `events()`, and `waitForFinish()`.
- Websocket usage is more explicit. Use `client.ws()` for a reusable session or
  `client.events.*` for one-shot subscriptions.

## Constructor And Auth

Legacy:

```ts
import { Client } from "@space-operator/client";

const client = new Client({
  host: "http://localhost:8080",
  token: Deno.env.get("APIKEY"),
  anonKey: Deno.env.get("ANON_KEY"),
});
```

New:

```ts
import { apiKeyAuth, createClient } from "@space-operator/client-next";

const client = createClient({
  baseUrl: "http://localhost:8080",
  auth: apiKeyAuth(Deno.env.get("APIKEY")!),
  anonKey: Deno.env.get("ANON_KEY"),
});
```

### No More Mutable Auth

Legacy:

```ts
client.setToken(nextToken);
```

New:

```ts
import { bearerAuth } from "@space-operator/client-next";

const authed = client.withAuth(bearerAuth(nextToken));
```

Construct a new client up front for different `fetch`, `logger`, `retry`, or
`timeoutMs` behavior instead of mutating an existing instance.

## Method Mapping

| Legacy                                                           | New                                                                              | Notes                                                                      |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| `new Client({ host, token, anonKey })`                           | `createClient({ baseUrl, auth, anonKey })`                                       | Auth is explicit via helpers like `apiKeyAuth(...)` and `bearerAuth(...)`. |
| `client.setToken(token)`                                         | `client.withAuth(...)`                                                           | Returns a new client view with different auth.                             |
| `client.setFetch(fetch)`                                         | `createClient({ fetch })`                                                        | Configure at construction time.                                            |
| `client.setLogger(logger)`                                       | `createClient({ logger })`                                                       | Configure at construction time.                                            |
| `client.initAuth(pubkey)`                                        | `client.auth.init(pubkey)`                                                       | Same backend route.                                                        |
| `client.confirmAuth(msg, signature)`                             | `client.auth.confirm(msg, signature)`                                            | Same backend route.                                                        |
| `client.claimToken()`                                            | `client.auth.claimToken()`                                                       | Same auth claim path.                                                      |
| `client.startFlow(id, params)`                                   | `client.flows.start(id, params)`                                                 | Returns `FlowRunHandle`, not a raw DTO.                                    |
| `client.startFlowShared(id, params)`                             | `client.flows.startShared(id, params)`                                           | Returns `FlowRunHandle`.                                                   |
| `client.startFlowUnverified(id, publicKey, params)`              | `client.flows.startUnverified(id, params, { publicKey })`                        | `publicKey` moves into options.                                            |
| `client.getFlowOutput(runId, token?)`                            | `client.flows.output(runId, { auth })` or `run.output()`                         | Prefer the handle form when you already started the run.                   |
| `client.getSignatureRequest(runId, token?)`                      | `client.flows.signatureRequest(runId, { auth })` or `run.signatureRequest()`     | The new API polls through transient `404`s for you.                        |
| `client.stopFlow(runId, params)`                                 | `client.flows.stop(runId, params)` or `run.stop(params)`                         | Prefer the handle form.                                                    |
| `client.submitSignature(params)`                                 | `client.signatures.submit(params)`                                               | Same route, namespaced API.                                                |
| `client.signAndSubmitSignature(req, publicKey, signTransaction)` | `signAndSubmitSignature(client.signatures, req, { publicKey, signTransaction })` | Helper moved to the Solana subpath.                                        |
| `client.deployFlow(id)`                                          | `client.flows.deploy(id)`                                                        | Same route.                                                                |
| `client.startDeployment(spec, params, token?)`                   | `client.deployments.start(spec, params, { auth })`                               | Returns `FlowRunHandle`.                                                   |
| `client.export()`                                                | `client.data.export()`                                                           | Namespaced.                                                                |
| `client.upsertWallet(body)`                                      | `client.wallets.upsert(body)`                                                    | Namespaced.                                                                |
| `client.ws()`                                                    | `client.ws()`                                                                    | Returns `WebSocketSession`, not `WsClient`.                                |
| `ws.subscribeFlowRunEvents(...)`                                 | `ws.subscribeFlowRun(...)` or `run.events()`                                     | `run.events()` is usually simpler.                                         |
| `ws.subscribeSignatureRequest(...)`                              | `ws.subscribeSignatureRequests(...)`                                             | Requires bearer/api-key websocket auth.                                    |

## Common Migrations

### Start A Flow And Read Output

Legacy:

```ts
const started = await client.startFlow(flowId, {
  inputs: { a: 1, b: 2 },
});
const output = await client.getFlowOutput(started.flow_run_id);
```

New:

```ts
const run = await client.flows.start(flowId, {
  inputs: { a: 1, b: 2 },
});
const output = await run.output();
```

### Unverified Start

Legacy:

```ts
const started = await client.startFlowUnverified(flowId, publicKey, {
  inputs: { a: 1 },
});
const output = await client.getFlowOutput(
  started.flow_run_id,
  started.token,
);
```

New:

```ts
const run = await client.flows.startUnverified(
  flowId,
  { inputs: { a: 1 } },
  { publicKey },
);
const output = await run.output();
```

Because `FlowRunHandle` already carries the returned token, you usually do not
need to thread it around manually anymore.

### Deployment Start And Signature Submission

Legacy:

```ts
const started = await client.startDeployment({ id: deploymentId }, params);
const req = await client.getSignatureRequest(started.flow_run_id);
await client.signAndSubmitSignature(req, keypair.publicKey, async (tx) => {
  tx.sign([keypair]);
  return tx;
});
const output = await client.getFlowOutput(started.flow_run_id);
```

New:

```ts
import { signAndSubmitSignature } from "@space-operator/client-next/solana";

const run = await client.deployments.start({ id: deploymentId }, params);
const req = await run.signatureRequest();
await signAndSubmitSignature(client.signatures, req, {
  publicKey: keypair.publicKey,
  signTransaction: async (tx) => {
    tx.sign([keypair]);
    return tx;
  },
});
const output = await run.output();
```

### Wallet Login

Legacy:

```ts
const msg = await client.initAuth(publicKey);
const auth = await client.confirmAuth(msg, signature);
```

New:

```ts
const auth = await client.auth.loginWithSignature(
  publicKey,
  signMessage,
);
```

`client.auth.init(...)` and `client.auth.confirm(...)` still exist when you need
the lower-level steps directly.

## Websocket Migration

### Reusable Session

Legacy:

```ts
const ws = client.ws();
await ws.authenticate();
await ws.subscribeFlowRunEvents((event) => {
  console.log(event.event);
}, runId);
```

New:

```ts
const ws = client.ws();
await ws.authenticate();

const events = await ws.subscribeFlowRun(runId);
for await (const event of events) {
  console.log(event.event);
}
```

### Prefer `run.events()` For Per-Run Streaming

If you already have a `FlowRunHandle`, this is usually the simplest path:

```ts
const run = await client.flows.start(flowId, params);

for await (const event of await run.events()) {
  console.log(event.event);
}
```

### Important Auth Difference

`publicKeyAuth(...)` works for unverified HTTP starts, but it does not produce a
user-authenticated websocket token for `subscribeSignatureRequests()`. For those
cases:

- use `run.events()` / `subscribeFlowRun(...)` to observe signature requests on
  that specific run, or
- use bearer/api-key auth when you need user-wide signature request
  subscriptions.

## Values

`Value` and `IValue` are still exported.

Legacy:

```ts
import { Value } from "@space-operator/client";
```

New:

```ts
import { Value } from "@space-operator/client-next";
```

The new client also accepts plain JSON-like inputs and normalizes them
internally, so you do not need to manually wrap everything in `Value`.

## x402 Compatibility

`@space-operator/client-next/x402` uses the compatibility shim in
[@space-operator/x402-fetch](/home/amir/code/space-operator/flow-backend/@space-operator/x402-fetch/src/mod.ts).

In that code, `legacy` means the currently published JavaScript `x402` package
shape that `PaymentRequirementsSchema` still expects:

- network names like `solana-devnet`
- `maxAmountRequired` instead of `amount`
- top-level `resource`, `description`, and `mimeType`
- optional object-valued `outputSchema`

The backend is already returning a newer payment-requirements shape in some
cases, including:

- CAIP-2 style chain ids like `solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1`
- `amount` instead of `maxAmountRequired`
- `resource` metadata carried separately
- `outputSchema: null`

The shim normalizes the backend response into the older JavaScript schema so the
existing `x402` client libraries can still build payment headers without failing
on schema validation.

## What To Change First

If you are migrating incrementally, the safest order is:

1. Replace `new Client(...)` with `createClient(...)`.
2. Replace mutable auth changes with `withAuth(...)`.
3. Replace direct flow/deployment DTO handling with `FlowRunHandle`.
4. Replace `WsClient` callbacks with `run.events()` or `WebSocketSession`.
5. Move Solana signing helpers to `@space-operator/client-next/solana`.
