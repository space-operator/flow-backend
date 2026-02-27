# V2-Only Refactor: Complete Tasks and Next Steps (flow-backend)

Date: 2026-02-20  
Scope: strict V2 execution path in this repo (`flow-backend`) with UUID flow IDs and V2 transport/contracts.

## Goal

Move execution and persistence to V2-only behavior:

1. `FlowId` is UUID end-to-end.
2. runtime loads come from `flows_v2` only.
3. ingest contract is V2 (`SpoNode`/`SpoEdge` + IValue-tagged config).
4. scoped node IDs remain intact; aliasing is explicit and scoped-aware.
5. no V1 execute fallback behavior.
6. command definition authoring/runtime uses the same V2 node-definition contract.

## Current Snapshot

### Completed

1. Core UUID migration is in place in `lib/flow-lib/src/config/mod.rs` (`FlowId = Uuid`).
2. V2 transport structs and conversion path are present in `lib/flow-lib/src/config/client.rs` and `lib/flow-lib/src/config/mod.rs` (`FlowConfig::from_v2`).
3. Scoped alias handling and IValue parsing helpers exist in `lib/flow-lib/src/command/mod.rs`.
4. Special command updates landed in non-`cmds-*` flow commands:
   - `crates/flow/src/command/flow_input.rs`
   - `crates/flow/src/command/flow_output.rs`
   - `crates/flow/src/command/interflow.rs`
   - `crates/flow/src/command/wallet.rs`
   - `crates/flow/src/command/rhai.rs`
5. DB access now reads `flows_v2` in core flow paths (`crates/db/src/connection/conn_impl/flows.rs`).
6. UUID deployment mappings are active in `crates/db/src/connection/conn_impl/deployments.rs`.
7. Flow server API routes are UUID-based (`crates/flow-server/src/api/*.rs` where flow IDs are parsed as `FlowId`).
8. Schema files were updated to stricter V2 contracts:
   - `schema/flow.schema.json`
   - `schema/node-definition.schema.json`
   - `schema/node-v2.schema.json`
   - `schema/nodes/*.jsonc` examples
9. Baseline squash folder was added for fresh DB setup:
   - `docker/supabase/squash/migrations/20260221000000_baseline_v2.sql`
   - `docker/supabase/squash/README.md`

### Still Open

1. migrate legacy command definition files in `crates/cmds-std/node-definitions/*.json` to the V2 schema contract.
2. migrate `CmdBuilder` off legacy `sources/targets/data.node_id` parsing to V2 definition parsing.
3. remove remaining compatibility shims that are no longer needed for strict V2.
4. finalize schema generation/docs workflow (`schema/render.ts` + `schema/llm-context.txt`) after Deno lock/version alignment.
5. run full test matrix and add missing regression coverage.
6. define operational rollout checklist (migration source, smoke suite, rollback plan).

## Complete Task List (File-by-File)

## 1) Schema Contract Hardening

1. Keep `schema/node-definition.schema.json` as canonical and make `schema/node-v2.schema.json` a thin alias/ref.
2. Expand example node contracts in `schema/nodes/` for all critical special nodes:
   - `interflow`
   - `interflow_instructions`
   - `wallet`
   - `rhai_script`
   - `deno_script`
   - `kv_explorer`
   - `file_explorer`
3. Update `schema/context.md` wording to reflect strict V2-only assumptions and no legacy form shape.
4. Regenerate `schema/llm-context.txt` once Deno lock compatibility is resolved (`schema/render.ts` path).

## 2) Node Definition Source Migration (Missing Step)

1. Migrate existing command definition source files from legacy format to V2 format:
   - source: `crates/cmds-std/node-definitions/*.json`
   - target/canonical: `schema/nodes/*.jsonc` (then generated/consumed artifact as needed)
2. Define and enforce field mapping from legacy -> V2:
   - `data.node_id` -> `name`
   - node publisher -> `author_handle`
   - `data.resources.source_code_url` -> `source_code`
   - `sources` -> `ports.outputs`
   - `targets` -> `ports.inputs`
   - `targets_form.json_schema` -> `config_schema`
   - target default values -> `config.<input_name>` (IValue-tagged; no `form_data` / `ui_schema`)
3. Add migration tooling (script/check) so conversion is repeatable, not manual one-off.
4. Add CI/schema validation step to fail if node definitions diverge from `schema/node-v2.schema.json`.

## 3) CmdBuilder Migration to V2

1. Update parsing types in `lib/flow-lib/src/config/node.rs` to support V2 node-definition contract.
2. Update `lib/flow-lib/src/command/builder.rs`:
   - build command name from V2 definition identity (`name` / scoped mapping policy)
   - build inputs from `ports.inputs`
   - build outputs from `ports.outputs`
   - keep permissions/instruction metadata behavior explicit
3. Update `flow_lib::node_definition!` consumption sites that currently expect legacy shape.
4. Remove legacy-only assumptions (`data.node_id`, `sources`, `targets`, `targets_form.*`) once all callers are migrated.
5. Add builder tests for V2 definitions to ensure parity with previous runtime behavior.

## 4) DB and Migration Policy

1. Confirm migration source per environment:
   - full chain: `docker/supabase/migrations`
   - fresh baseline: `docker/supabase/squash/migrations`
2. Document the chosen compose mount strategy in `docker/docker-compose.yml` usage docs.
3. Audit baseline SQL for legacy `flows` dependencies that are not required for strict V2 new deployments.
4. Validate UUID cutover migrations in order:
   - `20260220090200_flow_id_uuid_cutover.sql`
   - `20260220090300_deployments_uuid_cutover.sql`
   - `20260220090400_interflow_payload_uuid.sql`
   - `20260220090500_flow_x402_fees_uuid_cutover.sql`

## 5) flow-lib Strictness Cleanup

1. Decide and apply strict parsing mode in `lib/flow-lib/src/command/mod.rs`:
   - keep `parse_value_tagged_or_json` only for explicit non-strict call sites, or
   - remove it from strict V2 paths and enforce tagged-only reads.
2. Remove transport compatibility conversions if no longer required:
   - `impl From<ClientConfigV2> for ClientConfig`
   - `impl From<FlowRowV2> for FlowRow`
3. Keep `FlowConfig::from_v2` as the only V2-to-runtime conversion entrypoint.
4. Add/keep tests for:
   - scoped alias behavior (`@spo/*` only),
   - non-collapsing of non-`@spo/*` scopes,
   - IValue tag decoding matrix.

## 6) Flow Command Runtime (Non-`cmds-*`)

1. Enforce tagged-only config expectations with clear errors in:
   - `crates/flow/src/command/flow_input.rs`
   - `crates/flow/src/command/flow_output.rs`
   - `crates/flow/src/command/interflow.rs`
   - `crates/flow/src/command/wallet.rs`
2. Verify `interflow_instructions` command parity with UUID `flow_id` config.
3. Ensure Rhai matcher behavior remains strict for scoped and plain IDs in `crates/flow/src/command/rhai.rs`.

## 7) DB Connection Layer

1. Verify `row_to_flow_row` and conversion path in `crates/db/src/connection/conn_impl/flows.rs` do not silently drop required V2 runtime fields.
2. Remove stale export cleanup fields that are no longer part of strict V2 schema where appropriate:
   - `crates/db/src/connection/conn_impl.rs` (`flows.drop_in_place(...)` list).
3. Keep clone behavior strict for V2 config keys (`flow_id`, wallet tagged values) in `clone_flow_impl`.
4. Add regression tests for `get_flow_impl`, `get_flow_info_impl`, `get_flow_config_impl` with UUID IDs only.

## 8) flow-server API/Worker

1. Add API tests for invalid UUID handling with explicit 400 errors in:
   - `crates/flow-server/src/api/start_flow.rs`
   - `crates/flow-server/src/api/start_flow_shared.rs`
   - `crates/flow-server/src/api/start_flow_unverified.rs`
   - `crates/flow-server/src/api/deploy_flow.rs`
   - `crates/flow-server/src/api/clone_flow.rs`
   - `crates/flow-server/src/api/start_deployment.rs`
2. Verify worker message flow remains UUID-native:
   - `crates/flow-server/src/db_worker/messages.rs`
   - `crates/flow-server/src/db_worker/user_worker.rs`

## 9) @space-operator Client (In-Repo Package)

1. Keep `FlowId` as `string` in:
   - `@space-operator/client/src/types/common.ts`
   - `@space-operator/flow-lib/src/common.ts`
2. Validate start/deploy client calls remain UUID path-based in `@space-operator/client/src/client.ts`.
3. Maintain integration test UUID env usage through shared helper:
   - `@space-operator/client/integration_tests/utils.ts`
4. Update CI docs/env templates to list all required UUID test flow IDs.

## 10) External Dependency Track (Outside This Repo)

1. flow2 API/UI execute route cutover to `/api/v2/flows/{uuid}/execute` only.
2. SPO client hook cleanup in flow2 repo (remove V1 execute fallback branches).
3. Node-definition sync pipeline validation against new `config_schema` contracts.

## Next Steps (Recommended Order)

1. Freeze schema contracts: finalize `schema/node-definition.schema.json` as canonical and lock example nodes for all special types.
2. Migrate current command node definitions (`crates/cmds-std/node-definitions`) to V2 contract and validate with schema checks.
3. Migrate `CmdBuilder` to consume V2 node definitions directly.
4. Run DB dry-run on a fresh DB from squash baseline, then run smoke queries for `flows_v2`, `flow_run`, `flow_deployments*`.
5. Remove remaining strictness ambiguities in `flow-lib` (`parse_value_tagged_or_json` usage and V2->V1 conversion shims).
6. Add regression tests around flow command config parsing and interflow UUID behavior.
7. Add DB-layer tests for `flows_v2` read/clone paths and runtime field mapping.
8. Add/expand flow-server API UUID error-path tests.
9. Execute integration tests in `@space-operator/client/integration_tests` with UUID flow fixtures.
10. Run workspace test suite and publish a cutover readiness report.

## Verification Checklist

1. `cargo test --workspace` passes.
2. All flow-server start/deploy/clone endpoints accept UUID IDs and reject malformed IDs.
3. `flows_v2` is the only runtime flow source in DB read paths.
4. Interflow config accepts only tagged UUID string `flow_id`.
5. Wallet/flow_input/flow_output parse V2 tagged config as expected.
6. Integration tests in `@space-operator/client/integration_tests` pass with UUID env vars.
7. `crates/cmds-std/node-definitions` are fully migrated/validated against V2 node schema.
8. `CmdBuilder` consumes V2 node definitions without legacy field dependency.

## Definition of Done

1. No execution path depends on V1 flow transport or numeric flow IDs.
2. Runtime data model, API, DB mappings, and tests are UUID + V2-native.
3. Schema contracts and sample node definitions are aligned and reproducible.
4. Fresh DB bootstrapping path is documented and repeatable.
