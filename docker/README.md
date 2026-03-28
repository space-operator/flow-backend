# Generate secrets and configurations

Our Docker Compose setup needs 2 configuration files, both are located in `flow-backend/docker` folder:

- `.env`: dotenv file containing environment variables
- `.config.toml`: configuration file used by flow-server.

Template for these files are in `env.example` and `flow-server-config.toml`.

Generate secrets and config files for your server:
```bash
./gen-secrets.ts
```

Generated secrets are saved in `.env` and `.config.toml` files.

The script use `env.example` and `flow-server-config.toml` as templates,
you can edit them before running the script to customize values.

# Running

Start and wait for containers to be ready:

```bash
docker compose up -d --wait
```

Port binding:
- Supabase: port 8000
- Flow server: port 8080
- PostgreSQL: port 5432

To see Supabase Dashboard:

Open `.env` file to see `DASHBOARD_USERNAME` and `DASHBOARD_PASSWORD` values:

```bash
cat .env | grep DASHBOARD
```

Visit http://localhost:8000/ .

# Export your data and use them in self-hosted server

Follow steps from [here](https://docs.spaceoperator.com/self-hosting/export-data-to-your-instance).

# Bootstrap local test fixtures

The repo includes a checked-in fixture export at
[export.json](./export.json)
with the historical sample flows used by the client integration and E2E tests.

After the local stack is running, import and verify those fixtures with:

```bash
deno run -A ./bootstrap-test-fixtures.ts
```

For the full verification path, make sure your shell has:

- `SERVICE_ROLE_KEY` so the script can inspect and patch fixture rows directly
- `APIKEY` so the script can probe owner-authenticated deploys and anonymous
  deployment starts against the real server API

This will skip the import if the sample flows are already present, and it now
also runs a fixture preflight that checks:

- the required historical sample flows are present
- Deno fixture nodes still have inline source/code
- interflow fixture nodes still reference existing flows
- the unverified-start path still works against the local server
- the anonymous deployment-start path still works against the local server

To run only the fixture/server preflight without importing again:

```bash
deno run -A ./bootstrap-test-fixtures.ts --preflight-only
```

# Stop and clean up

To stop services:

```bash
docker compose down
```

Stop and clean up all data:

```bash
docker compose down -v
```
