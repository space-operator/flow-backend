# `@space-operator/contracts`

Shared backend-facing contracts for the Space Operator TypeScript surface.

This package contains:

- Zod schemas for request and response payloads used by `client-next`
- websocket event contracts
- JSON Schema exports derived from those Zod schemas
- generated TypeScript types from `schema/flow-server.openapi.json`

## Why This Exists

`@space-operator/client-next` and higher-level LLM surfaces such as `flow2/mcp`
need one source of truth for backend payload shapes. This package is the shared
layer for that contract information.

## Exports

- Runtime Zod schemas from
  [src/client.ts](/home/amir/code/space-operator/flow-backend/@space-operator/contracts/src/client.ts)
- Generated OpenAPI-derived types from
  [src/generated/flow_server_openapi.ts](/home/amir/code/space-operator/flow-backend/@space-operator/contracts/src/generated/flow_server_openapi.ts)

## Development

From this package root:

```bash
deno task check
deno task test
```

To refresh generated OpenAPI-derived types, regenerate the server schema from
the repo root and then run the client task:

```bash
cargo run -p generate-schema
cd /home/amir/code/space-operator/flow-backend/@space-operator/client-next
deno task generate:openapi-types
```
