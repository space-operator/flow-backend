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

# Stop and clean up

To stop services:

```bash
docker compose down
```

Stop and clean up all data:

```bash
docker compose down -v
```
