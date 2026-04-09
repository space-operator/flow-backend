# Legacy Integration Test Fixtures

The legacy client integration tests depend on a fixed set of sample flows that
are not stored beside the tests as standalone JSON files.

Those fixtures live in [`../../docker/export.json`](../../docker/export.json)
and are imported into a local Flow Server stack for testing.

## Bootstrap

From `@space-operator/client`:

```bash
deno task test:bootstrap-fixtures
```

That imports and verifies the historical fixture flows used by the old test
suite, including:

- `Add`
- `Deno Add`
- `Collatz`
- `Interflow Instructions`
- `Consts`
- `API Input`
- `Transfer SOL`
- `Collatz-Core`
- `Simple Transfer`

## Run

After bootstrapping, run the legacy integration suite with:

```bash
deno task test:integration
```

These tests now assume a Deno 2 toolchain. The repo is pinned to Deno `2.6.4`.

The bootstrap step assumes your local stack is already running and that `.env`
contains a working `SERVICE_ROLE_KEY`. Owner-authenticated tests also still
need a valid `APIKEY` and keypair in your environment.
