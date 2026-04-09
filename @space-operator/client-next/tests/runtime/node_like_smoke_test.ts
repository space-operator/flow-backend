import { assertEquals } from "@std/assert";
import { bearerAuth, createClient } from "../../src/mod.ts";
import { createMockWebSocketFactory } from "../support/mock_websocket.ts";

Deno.test("node-like smoke: works with injected fetch and websocket factory", async () => {
  const client = createClient({
    baseUrl: "http://example.test",
    auth: bearerAuth("jwt-1"),
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

  assertEquals(await client.service.healthcheck(), { success: true });
  const subscription = await client.events.subscribeFlowRun("run-1");
  const event = await subscription.next();
  await subscription.close();
  assertEquals(event.value?.event, "FlowFinish");
});
