import { assertEquals } from "@std/assert";
import { bearerAuth, createClient } from "../../src/mod.ts";
import { createMockWebSocketFactory } from "../support/mock_websocket.ts";

interface RecordedSpan {
  name: string;
  attributes: Record<string, string | number | boolean>;
}

function createTracer() {
  const spans: RecordedSpan[] = [];
  return {
    spans,
    tracer: {
      startActiveSpan(
        name: string,
        run: (span: {
          setAttribute: (key: string, value: string | number | boolean) => void;
          setStatus: (_status: unknown) => void;
          recordException: (_error: unknown) => void;
          end: () => void;
        }) => Promise<unknown>,
      ) {
        const span: RecordedSpan = { name, attributes: {} };
        spans.push(span);
        return run({
          setAttribute(key, value) {
            span.attributes[key] = value;
          },
          setStatus() {},
          recordException() {},
          end() {},
        });
      },
    },
  };
}

Deno.test("telemetry records http and websocket spans", async () => {
  const telemetry = createTracer();
  const client = createClient({
    baseUrl: "http://example.test",
    auth: bearerAuth("jwt-1"),
    telemetry: {
      tracer: telemetry.tracer as never,
      attributes: {
        "space_operator.test": true,
      },
    },
    fetch: async () => Response.json({ success: true }),
    webSocketFactory: createMockWebSocketFactory((socket, message) => {
      if (message.method === "Authenticate") {
        socket.serverSend({ id: message.id, Ok: { user_id: "user-1" } });
        return;
      }
      if (message.method === "SubscribeFlowRunEvents") {
        socket.serverSend({ id: message.id, Ok: { stream_id: 1 } });
        socket.serverSend({
          stream_id: 1,
          event: "FlowFinish",
          data: {
            flow_run_id: "run-1",
            time: "now",
            not_run: [],
            output: { M: { ok: { B: true } } },
          },
        });
      }
    }),
  });

  await client.service.healthcheck();
  const subscription = await client.events.subscribeFlowRun("run-1");
  await subscription.next();
  await subscription.close();

  assertEquals(
    telemetry.spans.some((span) => span.name === "space_operator.http.request"),
    true,
  );
  assertEquals(
    telemetry.spans.some((span) =>
      span.name === "space_operator.ws.subscribe_flow_run"
    ),
    true,
  );
  assertEquals(
    telemetry.spans.every((span) =>
      span.attributes["space_operator.test"] === true
    ),
    true,
  );
});
