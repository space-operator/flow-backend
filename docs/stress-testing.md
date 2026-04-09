# Stress Testing

Use the dedicated `k6` harnesses under [tools/loadtest](/home/amir/code/space-operator/flow-backend/tools/loadtest/README.md) to generate controlled start traffic.

```bash
k6 run tools/loadtest/src/start-flow.js \
  -e BASE_URL=http://127.0.0.1:8080 \
  -e FLOW_ID=<flow-id> \
  -e AUTH_TOKEN="$AUTH_TOKEN" \
  -e VUS=25 \
  -e DURATION=60s
```

Or against deployments:

```bash
k6 run tools/loadtest/src/start-deployment.js \
  -e BASE_URL=http://127.0.0.1:8080 \
  -e DEPLOYMENT_ID=<deployment-id> \
  -e X_API_KEY="$X_API_KEY" \
  -e VUS=25 \
  -e DURATION=60s
```

The harness reports:

- `http_req_duration`
- `http_reqs`
- `checks`
- `loadtest_start_success_total`
- `loadtest_start_failures_total`
- `loadtest_start_unexpected_status_total`

## Recommended test matrix

1. Control-plane test
   Hit a trivial flow with low-log nodes. This isolates auth, DB run creation, graph build, and command init.

2. Wide DAG test
   Use a flow with many cheap parallel nodes. This shows executor and host CPU saturation.

3. Bun saturation test
   Use a wide Bun-heavy flow and increase concurrency until `flow_node_permit_wait_seconds{runtime="bun"}` rises sharply.

4. Log storm test
   Use nodes that emit many logs. Watch the DB copy-in queue and `event_lag`.

5. Replay pressure test
   Keep long-running flows open and attach subscribers mid-run. Watch replay size and buffered event gauges.

## Metrics to watch

Executor:

- `flow_graph_build_seconds`
- `flow_graph_run_seconds`
- `flow_command_init_seconds`
- `flow_node_permit_wait_seconds`
- `flow_node_run_seconds`
- `flow_instruction_batch_size`
- `flow_instruction_execute_seconds`

Replay and persistence:

- `flow_run_active`
- `flow_run_subscribers`
- `flow_run_buffered_events`
- `flow_run_subscribe_replay_events`
- `flow_run_events_total`
- `flow_run_save_to_db_chunk_events`
- `flow_run_logs_batch_rows`
- `flow_run_save_to_db_chunk_seconds`

Global DB log ingest:

- `flow_server_db_copy_in_pending_batches`
- `flow_server_db_copy_in_pending_rows`
- `flow_server_db_copy_in_chunk_batches`
- `flow_server_db_copy_in_chunk_rows`
- `flow_server_db_copy_in_seconds`
- `flow_server_db_copy_in_dropped_batches_total`
- `flow_server_db_copy_in_dropped_rows_total`

Existing useful metrics:

- `new_flow_run`
- `event_lag`
- `batch_nodes_insert_size`
- `after_insert_size`

## Reading the signals

- If `flow_graph_build_seconds` rises first, flow creation and command init are the limiter.
- If `flow_node_permit_wait_seconds` rises first for Bun or Rhai, the semaphore is the limiter.
- If `flow_run_buffered_events` grows with load, replay memory is accumulating faster than runs finish.
- If `flow_server_db_copy_in_pending_rows` climbs and stays high, the global log ingest loop is falling behind.
- If `event_lag` rises while request latency stays moderate, persistence is the bottleneck more than the HTTP layer.
