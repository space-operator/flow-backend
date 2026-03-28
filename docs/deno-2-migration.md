# Deno 2 Migration Notes

This repo now targets Deno `2.6.4`.

The version is pinned in three places:

- `./.tool-versions` for local asdf/mise-style toolchains
- GitHub Actions via `denoland/setup-deno`
- Docker images via `denoland/deno:2.6.4`

## Repo-specific breaking changes

### `package.json` changes npm resolution defaults in Deno 2

This repo has a root `package.json`. In Deno 2, that changes the default
`nodeModulesDir` mode from the old auto-install behavior to `"manual"`.

To keep the old workflow for packages that use `npm:` dependencies, each local
`deno.json` now sets:

```json
{
  "nodeModulesDir": "auto"
}
```

Without that setting, a fresh checkout can fail under Deno 2 unless someone has
already run `deno install` or otherwise prepared `node_modules`.

### Browser smoke tests now assume Deno 2

`@space-operator/client-next/tests/playwright` previously treated Deno 1.x as a
special case and skipped the browser bundling test. That fallback is gone.

If someone explicitly enables the Playwright suite on Deno 1.x now, the test
fails fast with a clear version error instead of silently skipping coverage.

### Docker dependency prewarming should use the Deno 2 path

`docker/webhook/Dockerfile` now uses:

```bash
deno install --entrypoint main.ts
```

instead of relying on the older `deno cache` flow.

## Suggested local upgrade commands

If you already have Deno installed:

```bash
deno upgrade --version 2.6.4
```

If you use asdf or mise and they read `.tool-versions`, installing the pinned
version from the repo root is enough.
