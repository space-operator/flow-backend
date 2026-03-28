import { assertEquals } from "@std/assert";
import { bearerAuth, createClient } from "../../src/mod.ts";
import { MockWebSocket } from "../support/mock_websocket.ts";

Deno.test("browser-like smoke: falls back to global WebSocket when available", async () => {
  const originalWebSocket = globalThis.WebSocket;
  Object.defineProperty(globalThis, "WebSocket", {
    value: class extends MockWebSocket {
      constructor(url: string) {
        super((socket, message) => {
          if (message.method === "Authenticate") {
            socket.serverSend({ id: message.id, Ok: { user_id: "user-1" } });
            return;
          }
          if (message.method === "SubscribeFlowRunEvents") {
            socket.serverSend({ id: message.id, Ok: { stream_id: 2 } });
            socket.serverSend({
              stream_id: 2,
              event: "FlowFinish",
              data: {
                flow_run_id: "run-1",
                time: "now",
                not_run: [],
                output: { M: { ok: { B: true } } },
              },
            });
          }
        });
        void url;
      }
    },
    configurable: true,
    writable: true,
  });

  try {
    const client = createClient({
      baseUrl: "http://example.test",
      auth: bearerAuth("jwt-1"),
      fetch: async () => Response.json({ success: true }),
    });

    const subscription = await client.events.subscribeFlowRun("run-1");
    const event = await subscription.next();
    await subscription.close();

    assertEquals(event.value?.event, "FlowFinish");
  } finally {
    Object.defineProperty(globalThis, "WebSocket", {
      value: originalWebSocket,
      configurable: true,
      writable: true,
    });
  }
});
