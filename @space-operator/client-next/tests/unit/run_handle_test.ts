import { assertEquals, assertRejects } from "@std/assert";
import {
  apiKeyAuth,
  createClient,
  FlowRunFailedError,
  flowRunTokenAuth,
  Value,
} from "../../src/mod.ts";
import { createMockWebSocketFactory } from "../support/mock_websocket.ts";

function readHeaders(init: unknown): Headers {
  return new Headers(
    (init as { headers?: HeadersInit } | undefined)?.headers,
  );
}

function unitTest(name: string, fn: () => Promise<void>) {
  Deno.test({
    name,
    sanitizeOps: false,
    sanitizeResources: false,
    fn,
  });
}

unitTest(
  "run handles use flow-run token auth by default and can be overridden",
  async () => {
    const seen: Array<
      { url: string; auth: string | null; apiKey: string | null }
    > = [];
    const client = createClient({
      baseUrl: "http://example.test",
      fetch: async (input, init) => {
        const headers = readHeaders(init);
        seen.push({
          url: String(input),
          auth: headers.get("authorization"),
          apiKey: headers.get("x-api-key"),
        });

        if (String(input).endsWith("/flow/start_unverified/flow-1")) {
          return Response.json({ flow_run_id: "run-1", token: "frt-1" });
        }
        return Response.json({ M: { ok: { B: true } } });
      },
    });

    const run = await client.flows.startUnverified("flow-1", {}, {
      publicKey: "PubKey1111111111111111111111111111111111",
    });
    const value = await run.output();
    const overriddenValue = await run.withAuth(apiKeyAuth("b3-api-key"))
      .output();

    assertEquals(value, new Value({ ok: true }));
    assertEquals(overriddenValue, new Value({ ok: true }));
    assertEquals(seen[1].auth, "Bearer frt-1");
    assertEquals(seen[1].apiKey, null);
    assertEquals(seen[2].apiKey, "b3-api-key");
  },
);

unitTest(
  "withAuth can replace handle auth without losing token metadata",
  async () => {
    const seen: Array<string | null> = [];
    const client = createClient({
      baseUrl: "http://example.test",
      fetch: async (input, init) => {
        const headers = readHeaders(init);
        seen.push(headers.get("authorization"));
        if (String(input).endsWith("/flow/start_unverified/flow-1")) {
          return Response.json({ flow_run_id: "run-1", token: "frt-1" });
        }
        return Response.json({ M: { ok: { B: true } } });
      },
    });

    const run = await client.flows.startUnverified("flow-1", {}, {
      publicKey: "PubKey1111111111111111111111111111111111",
    });
    const handle = run.withAuth(flowRunTokenAuth("frt-2"));
    await handle.output();

    assertEquals(handle.id, "run-1");
    assertEquals(handle.token, "frt-1");
    assertEquals(seen.at(-1), "Bearer frt-2");
  },
);

unitTest(
  "waitForFinish throws a typed flow failure when the run errors",
  async () => {
    const client = createClient({
      baseUrl: "http://example.test",
      webSocketFactory: createMockWebSocketFactory((socket, message) => {
        if (message.method === "Authenticate") {
          socket.serverSend({ id: message.id, Ok: { flow_run_id: "run-1" } });
          return;
        }
        if (message.method === "SubscribeFlowRunEvents") {
          socket.serverSend({ id: message.id, Ok: { stream_id: 4 } });
          socket.serverSend({
            stream_id: 4,
            event: "FlowError",
            data: {
              flow_run_id: "run-1",
              time: "now",
              error: "run exploded",
            },
          });
        }
      }),
      fetch: async () =>
        Response.json({ flow_run_id: "run-1", token: "frt-1" }),
    });

    const run = await client.flows.startUnverified("flow-1", {}, {
      publicKey: "PubKey1111111111111111111111111111111111",
    });
    const error = await assertRejects(
      () => run.waitForFinish({ auth: flowRunTokenAuth("frt-1") }),
      FlowRunFailedError,
      "run exploded",
    );

    assertEquals(error.details, {
      flow_run_id: "run-1",
      time: "now",
      error: "run exploded",
    });
  },
);
