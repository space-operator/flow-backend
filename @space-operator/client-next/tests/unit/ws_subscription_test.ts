import { assertEquals, assertRejects } from "@std/assert";
import {
  bearerAuth,
  createClient,
  flowRunTokenAuth,
  WebSocketProtocolError,
} from "../../src/mod.ts";
import {
  createMockWebSocketFactory,
  type MockWebSocket,
} from "../support/mock_websocket.ts";

function unitTest(name: string, fn: () => Promise<void>) {
  Deno.test({
    name,
    sanitizeOps: false,
    sanitizeResources: false,
    fn,
  });
}

unitTest(
  "flow run subscriptions authenticate and parse typed value payloads",
  async () => {
    const client = createClient({
      baseUrl: "http://example.test",
      auth: bearerAuth("jwt-1"),
      webSocketFactory: createMockWebSocketFactory((socket, message) => {
        if (message.method === "Authenticate") {
          socket.serverSend({ id: message.id, Ok: { user_id: "user-1" } });
          return;
        }
        if (message.method === "SubscribeFlowRunEvents") {
          socket.serverSend({ id: message.id, Ok: { stream_id: 7 } });
          socket.serverSend({
            stream_id: 7,
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

    const subscription = await client.events.subscribeFlowRun("run-1");
    const first = await subscription.next();
    await subscription.close();

    assertEquals(first.done, false);
    assertEquals(first.value?.event, "FlowFinish");
    assertEquals(first.value?.data.output.toJSObject(), { ok: true });
  },
);

unitTest(
  "run handles can wait for finish using flow-run token auth",
  async () => {
    const client = createClient({
      baseUrl: "http://example.test",
      webSocketFactory: createMockWebSocketFactory((socket, message) => {
        if (message.method === "Authenticate") {
          socket.serverSend({ id: message.id, Ok: { flow_run_id: "run-1" } });
          return;
        }
        if (message.method === "SubscribeFlowRunEvents") {
          socket.serverSend({ id: message.id, Ok: { stream_id: 9 } });
          socket.serverSend({
            stream_id: 9,
            event: "FlowFinish",
            data: {
              flow_run_id: "run-1",
              time: "now",
              not_run: [],
              output: { M: { count: { D: "3" } } },
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
    const finish = await run.waitForFinish({
      auth: flowRunTokenAuth("frt-1"),
    });

    assertEquals(finish.output.toJSObject(), { count: 3 });
  },
);

unitTest(
  "signature request subscriptions use the correct ws method",
  async () => {
    const methods: string[] = [];
    const client = createClient({
      baseUrl: "http://example.test",
      auth: bearerAuth("jwt-1"),
      webSocketFactory: createMockWebSocketFactory((socket, message) => {
        methods.push(String(message.method));
        if (message.method === "Authenticate") {
          socket.serverSend({ id: message.id, Ok: { user_id: "user-1" } });
          return;
        }
        if (message.method === "SubscribeSignatureRequests") {
          socket.serverSend({ id: message.id, Ok: { stream_id: 10 } });
          socket.serverSend({
            stream_id: 10,
            event: "SignatureRequest",
            data: {
              id: 1,
              time: "now",
              pubkey: "11111111111111111111111111111111",
              message: "",
              timeout: 10,
            },
          });
        }
      }),
    });

    const subscription = await client.events.subscribeSignatureRequests();
    const first = await subscription.next();
    await subscription.close();

    assertEquals(methods, ["Authenticate", "SubscribeSignatureRequests"]);
    assertEquals(first.value?.event, "SignatureRequest");
  },
);

unitTest(
  "protocol errors reject subscriptions instead of throwing out-of-band",
  async () => {
    const client = createClient({
      baseUrl: "http://example.test",
      auth: bearerAuth("jwt-1"),
      webSocketFactory: createMockWebSocketFactory((socket, message) => {
        if (message.method === "Authenticate") {
          socket.serverSend({ id: message.id, Ok: { user_id: "user-1" } });
          return;
        }
        if (message.method === "SubscribeFlowRunEvents") {
          socket.serverSend({ id: message.id, Ok: { stream_id: 11 } });
          socket.serverSend({ event: "invalid" });
        }
      }),
    });

    const ws = client.ws();
    await ws.authenticate();
    const subscription = await ws.subscribeFlowRun("run-1");
    await assertRejects(
      () => subscription.next(),
      WebSocketProtocolError,
      "invalid websocket payload",
    );
    await subscription.closed;
    await ws.close();
  },
);

unitTest(
  "reusable websocket sessions buffer early events and multiplex subscriptions",
  async () => {
    const methods: string[] = [];
    const client = createClient({
      baseUrl: "http://example.test",
      auth: bearerAuth("jwt-1"),
      webSocketFactory: createMockWebSocketFactory((socket, message) => {
        methods.push(String(message.method));
        if (message.method === "Authenticate") {
          socket.serverSend({ id: message.id, Ok: { user_id: "user-1" } });
          return;
        }
        if (message.method === "SubscribeFlowRunEvents") {
          socket.serverSend({ id: message.id, Ok: { stream_id: 21 } });
          socket.serverSend({
            stream_id: 21,
            event: "FlowFinish",
            data: {
              flow_run_id: "run-1",
              time: "now",
              not_run: [],
              output: { M: { ok: { B: true } } },
            },
          });
          return;
        }
        if (message.method === "SubscribeSignatureRequests") {
          socket.serverSend({ id: message.id, Ok: { stream_id: 22 } });
          socket.serverSend({
            stream_id: 22,
            event: "SignatureRequest",
            data: {
              id: 1,
              time: "now",
              pubkey: "11111111111111111111111111111111",
              message: "",
              timeout: 10,
            },
          });
        }
      }),
    });

    const ws = client.ws();
    const identity = await ws.authenticate();
    const flowSubscription = await ws.subscribeFlowRun("run-1");
    const signatureSubscription = await ws.subscribeSignatureRequests();
    const flowEvent = await flowSubscription.next();
    const signatureEvent = await signatureSubscription.next();

    await flowSubscription.close();
    await signatureSubscription.close();
    await ws.close();

    assertEquals(identity?.user_id, "user-1");
    assertEquals(methods, [
      "Authenticate",
      "SubscribeFlowRunEvents",
      "SubscribeSignatureRequests",
    ]);
    assertEquals(flowEvent.value?.event, "FlowFinish");
    assertEquals(flowEvent.value?.data.output.toJSObject(), { ok: true });
    assertEquals(signatureEvent.value?.event, "SignatureRequest");
  },
);

unitTest(
  "one-shot websocket helpers close failed sessions during handshake",
  async () => {
    const sockets: MockWebSocket[] = [];
    const client = createClient({
      baseUrl: "http://example.test",
      auth: bearerAuth("jwt-1"),
      webSocketFactory: createMockWebSocketFactory((socket, message) => {
        if (!sockets.includes(socket)) {
          sockets.push(socket);
        }
        if (message.method === "Authenticate") {
          socket.serverSend({ id: message.id, Err: "bad token" });
        }
      }),
    });

    await assertRejects(
      () => client.events.subscribeFlowRun("run-1"),
      WebSocketProtocolError,
      "bad token",
    );

    assertEquals(sockets.length, 1);
    assertEquals(sockets[0].closed, true);
  },
);

unitTest(
  "shared websocket sessions drop late messages from closed streams",
  async () => {
    let socketRef: MockWebSocket | undefined;
    let flowSubscribeCount = 0;
    const client = createClient({
      baseUrl: "http://example.test",
      auth: bearerAuth("jwt-1"),
      webSocketFactory: createMockWebSocketFactory((socket, message) => {
        socketRef = socket;
        if (message.method === "Authenticate") {
          socket.serverSend({ id: message.id, Ok: { user_id: "user-1" } });
          return;
        }
        if (message.method === "SubscribeFlowRunEvents") {
          flowSubscribeCount += 1;
          socket.serverSend({ id: message.id, Ok: { stream_id: 31 } });
          if (flowSubscribeCount === 2) {
            socket.serverSend({
              stream_id: 31,
              event: "FlowFinish",
              data: {
                flow_run_id: "run-2",
                time: "now",
                not_run: [],
                output: { M: { fresh: { B: true } } },
              },
            });
          }
        }
      }),
    });

    const ws = client.ws();
    await ws.authenticate();
    const firstSubscription = await ws.subscribeFlowRun("run-1");
    await firstSubscription.close();
    socketRef?.serverSend({
      stream_id: 31,
      event: "FlowFinish",
      data: {
        flow_run_id: "run-1",
        time: "stale",
        not_run: [],
        output: { M: { stale: { B: true } } },
      },
    });

    const secondSubscription = await ws.subscribeFlowRun("run-2");
    const secondEvent = await secondSubscription.next();

    await secondSubscription.close();
    await ws.close();

    assertEquals(secondEvent.value?.event, "FlowFinish");
    assertEquals(secondEvent.value?.data.output.toJSObject(), { fresh: true });
  },
);

unitTest(
  "shared websocket sessions reset the socket when switching to unauthenticated use",
  async () => {
    const sockets: MockWebSocket[] = [];
    const methodsBySocket: string[][] = [];
    const client = createClient({
      baseUrl: "http://example.test",
      auth: bearerAuth("jwt-1"),
      webSocketFactory: createMockWebSocketFactory((socket, message) => {
        let index = sockets.indexOf(socket);
        if (index === -1) {
          index = sockets.push(socket) - 1;
          methodsBySocket[index] = [];
        }
        methodsBySocket[index].push(String(message.method));
        if (message.method === "Authenticate") {
          socket.serverSend({ id: message.id, Ok: { user_id: "user-1" } });
          return;
        }
        if (message.method === "SubscribeFlowRunEvents") {
          socket.serverSend({ id: message.id, Ok: { stream_id: 40 } });
          socket.serverSend({
            stream_id: 40,
            event: "FlowFinish",
            data: {
              flow_run_id: "run-1",
              time: "now",
              not_run: [],
              output: { M: { unauthenticated: { B: true } } },
            },
          });
        }
      }),
    });

    const ws = client.ws();
    await ws.authenticate();
    await ws.authenticate(undefined);
    const subscription = await ws.subscribeFlowRun("run-1", {
      auth: undefined,
    });
    const event = await subscription.next();

    await subscription.close();
    await ws.close();

    assertEquals(sockets.length, 2);
    assertEquals(sockets[0].closed, true);
    assertEquals(methodsBySocket[0], ["Authenticate"]);
    assertEquals(methodsBySocket[1], ["SubscribeFlowRunEvents"]);
    assertEquals(event.value?.data.output.toJSObject(), {
      unauthenticated: true,
    });
  },
);
