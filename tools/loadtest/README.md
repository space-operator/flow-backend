# Load Testing

This package holds the dedicated `k6` harnesses for flow-server.

Scenarios:

- `src/start-flow.js`
- `src/start-deployment.js`

Run directly with `k6`:

```bash
k6 run tools/loadtest/src/start-flow.js \
  -e BASE_URL=http://127.0.0.1:8080 \
  -e FLOW_ID=<flow-id> \
  -e AUTH_TOKEN="$AUTH_TOKEN" \
  -e VUS=25 \
  -e DURATION=60s
```

```bash
k6 run tools/loadtest/src/start-deployment.js \
  -e BASE_URL=http://127.0.0.1:8080 \
  -e DEPLOYMENT_ID=<deployment-id> \
  -e X_API_KEY="$X_API_KEY" \
  -e VUS=25 \
  -e DURATION=60s
```

Useful env vars:

- `BASE_URL`
- `FLOW_ID`
- `DEPLOYMENT_ID`
- `AUTH_TOKEN`
- `X_API_KEY`
- `INPUTS_JSON`
- `ENVIRONMENT_JSON`
- `OUTPUT_INSTRUCTIONS=1`
- `VUS`
- `DURATION`
- `ITERATIONS`

`ITERATIONS` switches the scenario to a fixed shared-iterations run. Without it, the harness uses constant VUs for `DURATION`.
