# Testing `@space-operator/client-next`

This package has three test layers:

- `tests/unit` Fast, isolated checks for auth resolution, value normalization,
  websocket behavior, run handles, and Solana helper edge cases.
- `tests/runtime` Cross-runtime smoke tests for Deno, browser-like, and
  Node-like environments.
- `tests/playwright` Real browser websocket smoke coverage for the browser
  bundle path.
- `tests/contract` The live end-to-end suite. These tests talk to a real Flow
  Server stack and validate the client against actual backend behavior.

`tests/contract` is the package's E2E suite even though the folder still uses
the older `contract` name.

## Commands

Bootstrap the checked-in sample flows into a fresh local stack:

```bash
deno task test:bootstrap-fixtures
```

Run the fixture/server preflight without reimporting:

```bash
deno task test:preflight-fixtures
```

Run all non-live tests from the package root:

```bash
deno task test:ci
```

Or run the non-live layers directly:

```bash
deno task test:unit
deno task test:runtime
```

Run the live E2E suite:

```bash
RUN_SPACE_OPERATOR_E2E_TESTS=1 deno task test:e2e
```

The default live suite is expected to pass in the supported test environment. It
intentionally leaves the export and x402 cases ignored unless you opt in to
them.

Run the externally blocked live tests when the environment is ready:

```bash
RUN_SPACE_OPERATOR_E2E_TESTS=1 \
RUN_SPACE_OPERATOR_EXPORT_TESTS=1 \
RUN_SPACE_OPERATOR_X402_TESTS=1 \
deno task test:e2e
```

On a fresh local stack, run `deno task test:bootstrap-fixtures` once before the
live suite so the historical sample flows exist locally. On later runs,
`deno task test:preflight-fixtures` is a faster way to catch broken fixture data
or local auth/server drift before the E2E suite.

The older flag still works too:

```bash
RUN_SPACE_OPERATOR_CONTRACT_TESTS=1 deno task test:contract
```

Run the Playwright browser smoke test:

```bash
RUN_SPACE_OPERATOR_PLAYWRIGHT_TESTS=1 deno task test:playwright
```

Run a single live file:

```bash
RUN_SPACE_OPERATOR_E2E_TESTS=1 deno test --no-check -A tests/contract/kv_test.ts
```

## Required Environment

These variables are used by the live suite:

- `RUN_SPACE_OPERATOR_E2E_TESTS=1` Enables the gated live tests.
- `RUN_SPACE_OPERATOR_EXPORT_TESTS=1` Enables the export contract test, which
  depends on a stable backend COPY-OUT connection.
- `RUN_SPACE_OPERATOR_X402_TESTS=1` Enables the paid deployment contract test,
  which depends on a funded devnet signer.
- `FLOW_SERVER_URL` Defaults to `http://localhost:8080`.
- `SUPABASE_URL` Defaults to `http://localhost:8000`.
- `ANON_KEY` Required for auth bootstrap and Supabase session validation.
- `APIKEY` Required for owner-authenticated API calls.
- `KEYPAIR` Required for auth, deployment, and x402 tests. This should be a
  base58-encoded secret key. The shared helper also falls back to lowercase
  `keypair` and `OWNER_KEYPAIR` when present.
- `SOLANA_DEVNET_URL` Required for the deployment action-signer test that sends
  a real devnet transaction.

The default live suite intentionally skips two known non-client blockers:

- the export contract test
- the x402 contract test

Those tests are still available through the opt-in flags above. This keeps the
default E2E pass/fail signal focused on SDK health rather than fixture, funding,
or remote database infrastructure.

The live fixture flow IDs can also be overridden per environment:

- `START_FLOW_ID`
- `DENO_FLOW_ID`
- `INTERFLOW_FLOW_ID`
- `INTERFLOW_INSTRUCTIONS_FLOW_ID`
- `CONSTS_FLOW_ID`
- `API_INPUT_FLOW_ID`
- `DEPLOY_RUN_FLOW_ID`
- `DEPLOY_DELETE_FLOW_ID`
- `DEPLOY_ACTION_FLOW_ID`
- `DEPLOY_SIMPLE_FLOW_ID`
- `X402_FLOW_ID`

If unset, the suite first tries to resolve fixtures by flow name from the
owner's visible flows and only then falls back to the historical hardcoded IDs.
If neither is present, the suite now points you at
`deno task test:bootstrap-fixtures`.

`test:bootstrap-fixtures` and `test:preflight-fixtures` also require:

- `SERVICE_ROLE_KEY` so the script can validate imported fixture rows and probe
  the unverified-start path directly against the local stack.
- `APIKEY` so the script can create a temporary deployment and probe the
  anonymous deployment-start path through the real server API.

For `tests/playwright`:

- `RUN_SPACE_OPERATOR_PLAYWRIGHT_TESTS=1` Enables the gated browser smoke test.
- Deno `2.6.4+` is the supported baseline for the browser bundling toolchain
  used by that test. The suite no longer treats Deno 1.x as supported.

## Coverage Map

The live suite currently covers every public namespace:

- `auth` Login bootstrap, confirm, and new-user creation in
  `tests/contract/auth_test.ts`.
- `service` `/info` and `/healthcheck` in `tests/contract/service_test.ts`.
- `flows` Start, output, interflow behavior, clone, `startShared`, and
  `startUnverified` in `tests/contract/flow_test.ts`.
- `deployments` Deployment creation and the main deployment execution paths in
  `tests/contract/deployment_test.ts`.
- `events` Flow-run subscriptions and signature-request subscriptions in
  `tests/contract/api_input_test.ts` and `tests/contract/events_test.ts`.
- `signatures` Real signature submission flows through the deployment tests.
- `wallets` Direct wallet upsert coverage in `tests/contract/wallets_test.ts`.
- `apiKeys` Create, inspect, and delete coverage in
  `tests/contract/api_keys_test.ts`.
- `kv` Store and item CRUD coverage in `tests/contract/kv_test.ts`.
- `data` Export coverage in `tests/contract/export_test.ts`. This is opt-in
  while the remote COPY-OUT path is unstable.
- `x402` Paid deployment start coverage in `tests/contract/x402_test.ts`. This
  is opt-in because it requires a funded devnet signer.

## State And Cleanup

The live suite creates and mutates real backend state. Tests clean up after
themselves where practical, but they still exercise real resources:

- API keys are created and deleted.
- KV stores are created and deleted.
- Wallet rows are inserted and then deleted.
- Flow clone tests create cloned flows and then delete them.
- Some flow tests temporarily toggle flags on a fixture flow and restore the
  original values.
- Deployment tests create real deployments and may touch devnet.

Run the live suite against disposable or well-understood test data, not a
production environment.

## Fixture Preflight

The bootstrap/preflight script now checks more than simple flow existence:

- fixture flows are present by name
- Deno fixture flows still have inline `source`/`code` in their node config
- interflow and interflow-instructions nodes still point at existing flow UUIDs
- the local server can still execute an unverified start against the `Add`
  fixture by temporarily enabling `start_unverified` and `is_public`
- the local server can still execute an anonymous deployment start against a
  temporary deployment of the `Add` fixture, which exercises the same public-key
  user-creation/login path used by the failing deployment tests
- the `API Input` fixture still completes through both direct submit and webhook
  mode
- the deployment fixture used by the signature tests still requests a signature
  from the owner keypair configured in `KEYPAIR` / `OWNER_KEYPAIR`

If those checks fail, the script exits early with a fixture/server diagnosis so
the E2E suite does not fail later with opaque runtime errors.

## Known External Blockers

The following cases are currently outside the client harness itself:

- `data export contract` This depends on the backend keeping the database
  COPY-OUT connection alive for the full export stream.
- `x402 contract: start deployment with wrapped fetch` This depends on a funded
  devnet signer that can pay the on-chain x402 fee.

The default `RUN_SPACE_OPERATOR_E2E_TESTS=1 deno task test:e2e` path leaves
those two cases ignored so the suite remains a stable SDK regression signal.

## When Adding New E2E Coverage

- Prefer using helpers from `tests/contract/_shared.ts` instead of duplicating
  auth and Supabase setup.
- Use unique resource names via `randomName(...)` or `randomStoreName(...)`.
- Clean up server state in `finally` blocks whenever the test creates durable
  records.
- Keep live tests inside `tests/contract` so `deno task test:e2e` remains the
  single entrypoint for full-stack verification.
