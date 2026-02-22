import {
  Context,
  Duration,
  Effect,
  Layer,
  Queue,
  Ref,
  Schedule,
  Stream,
  Redacted,
} from "effect";
import { SpaceOperatorConfig } from "./Config.ts";
import {
  AuthTokenError,
  WsConnectionError,
  WsProtocolError,
  WsTimeoutError,
} from "./Errors.ts";
import {
  type AuthenticateResponseOk,
  type FlowRunEvent,
  type SignatureRequestsEvent,
  makeAuthenticateRequest,
  makeSubscribeFlowRunEventsRequest,
  makeSubscribeSignatureRequestsRequest,
} from "./Schema/Ws.ts";

// --- Connection State ---

export type WsConnectionState =
  | "disconnected"
  | "connecting"
  | "authenticating"
  | "connected"
  | "resubscribing"
  | "waiting_to_reconnect";

// --- Options ---

export interface WsServiceOptions {
  /** Max reconnect attempts. 0 = infinite. Default: 10. */
  maxRetries?: number;
  /** Base delay in ms for exponential backoff. Default: 1000. */
  baseDelay?: number;
  /** Max delay cap in ms. Default: 30000. */
  maxDelay?: number;
  /** Connection timeout in ms. Default: 10000. */
  connectTimeout?: number;
  /** Per-request timeout in ms. Default: 30000. */
  requestTimeout?: number;
  /** Heartbeat interval in ms. Default: 30000. Set to 0 to disable. */
  heartbeatInterval?: number;
  /** Pong timeout in ms. Default: 10000. */
  pongTimeout?: number;
  /** Callback for state changes. */
  onStateChange?: (state: WsConnectionState) => void;
}

// --- Internal Types ---

interface PendingRequest {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
}

interface SubscriptionEntry {
  clientSubId: number;
  serverStreamId: number | null;
  type: "FlowRunEvents" | "SignatureRequests";
  flowRunId?: string;
  token?: string;
  queue: Queue.Queue<unknown>;
  completed: boolean;
}

// --- Service Definition ---

export interface WsServiceShape {
  /** Open the WebSocket connection. */
  readonly connect: () => Effect.Effect<
    void,
    WsConnectionError | WsTimeoutError
  >;

  /** Authenticate over the open connection. */
  readonly authenticate: () => Effect.Effect<
    AuthenticateResponseOk,
    WsProtocolError | WsTimeoutError | AuthTokenError | WsConnectionError
  >;

  /** Subscribe to flow run events. Returns a Stream that survives reconnects. */
  readonly subscribeFlowRunEvents: (
    flowRunId: string,
    token?: string,
  ) => Stream.Stream<FlowRunEvent, WsProtocolError | WsConnectionError>;

  /** Subscribe to signature requests. Returns a Stream that survives reconnects. */
  readonly subscribeSignatureRequests: () => Stream.Stream<
    SignatureRequestsEvent,
    WsProtocolError | WsConnectionError
  >;

  /** Close the connection intentionally (no reconnect). */
  readonly close: () => Effect.Effect<void>;

  /** Current connection state. */
  readonly state: Effect.Effect<WsConnectionState>;
}

export class WsService extends Context.Tag("WsService")<
  WsService,
  WsServiceShape
>() {}

// --- Implementation ---

export const WsServiceLive: Layer.Layer<
  WsService,
  never,
  SpaceOperatorConfig
> = Layer.effect(
  WsService,
  Effect.gen(function* () {
    const config = yield* SpaceOperatorConfig;

    // Derive WS URL from host
    const wsUrl =
      config.wsUrl ?? config.host.replace(/^http/, "ws") + "/ws";

    // State
    const stateRef = yield* Ref.make<WsConnectionState>("disconnected");
    const connRef = yield* Ref.make<WebSocket | null>(null);
    const nextIdRef = yield* Ref.make(1);
    const pendingRef = yield* Ref.make(
      new Map<number, PendingRequest>(),
    );
    const subsRef = yield* Ref.make(new Map<number, SubscriptionEntry>());
    const nextSubIdRef = yield* Ref.make(1);
    const intentionalCloseRef = yield* Ref.make(false);

    // Options (defaults)
    const opts: Required<WsServiceOptions> = {
      maxRetries: 10,
      baseDelay: 1000,
      maxDelay: 30000,
      connectTimeout: 10000,
      requestTimeout: 30000,
      heartbeatInterval: 30000,
      pongTimeout: 10000,
      onStateChange: () => {},
    };

    const setState = (s: WsConnectionState) =>
      Ref.set(stateRef, s).pipe(
        Effect.tap(() => Effect.sync(() => opts.onStateChange(s))),
      );

    const nextId = () =>
      Ref.getAndUpdate(nextIdRef, (n) => n + 1);

    // --- Send a WS message and wait for response ---

    const send = <T>(
      msg: unknown,
      id: number,
    ): Effect.Effect<T, WsConnectionError | WsTimeoutError> =>
      Effect.gen(function* () {
        const ws = yield* Ref.get(connRef);
        const currentState = yield* Ref.get(stateRef);
        const canSend = ws !== null &&
          (currentState === "connected" ||
            currentState === "authenticating" ||
            currentState === "resubscribing");

        if (!canSend || ws === null) {
          return yield* Effect.fail(
            new WsConnectionError({ message: "WebSocket not connected" }),
          );
        }

        const result = yield* Effect.async<T, WsConnectionError>((resume) => {
          const pending: PendingRequest = {
            resolve: (v) => resume(Effect.succeed(v as T)),
            reject: (e) =>
              resume(
                Effect.fail(new WsConnectionError({ message: e.message })),
              ),
          };

          Effect.runSync(
            Ref.update(pendingRef, (m) => {
              const next = new Map(m);
              next.set(id, pending);
              return next;
            }),
          );

          ws.send(JSON.stringify(msg));
        });

        return result;
      }).pipe(
        Effect.timeoutFail({
          duration: Duration.millis(opts.requestTimeout),
          onTimeout: () =>
            new WsTimeoutError({ message: "WebSocket request timed out" }),
        }),
      );

    // --- Connect ---

    const doConnect = (): Effect.Effect<
      void,
      WsConnectionError | WsTimeoutError
    > =>
      Effect.gen(function* () {
        yield* setState("connecting");
        yield* Ref.set(intentionalCloseRef, false);

        const ws = yield* Effect.async<
          WebSocket,
          WsConnectionError
        >((resume) => {
          const socket = new WebSocket(wsUrl);

          socket.onopen = () => {
            resume(Effect.succeed(socket));
          };

          socket.onerror = (ev) => {
            resume(
              Effect.fail(
                new WsConnectionError({
                  message: `WebSocket connection failed: ${ev}`,
                }),
              ),
            );
          };

          socket.onmessage = (ev) => {
            handleMessage(ev.data);
          };

          socket.onclose = () => {
            handleClose();
          };
        }).pipe(
          Effect.timeoutFail({
            duration: Duration.millis(opts.connectTimeout),
            onTimeout: () =>
              new WsTimeoutError({
                message: "WebSocket connection timed out",
              }),
          }),
        );

        yield* Ref.set(connRef, ws);
      });

    // --- Message Handler ---

    const handleMessage = (raw: string | ArrayBuffer) => {
      try {
        const text = typeof raw === "string" ? raw : new TextDecoder().decode(raw);
        const json = JSON.parse(text);

        // Response to a pending request (has `id` field)
        if (typeof json.id === "number") {
          const pending = Effect.runSync(
            Ref.get(pendingRef).pipe(
              Effect.map((m) => m.get(json.id)),
            ),
          );
          if (pending) {
            Effect.runSync(
              Ref.update(pendingRef, (m) => {
                const next = new Map(m);
                next.delete(json.id);
                return next;
              }),
            );
            if (json.Err !== undefined) {
              pending.reject(new Error(json.Err));
            } else {
              pending.resolve(json.Ok ?? json);
            }
            return;
          }
        }

        // Stream event (has `stream_id` field)
        if (typeof json.stream_id === "number") {
          const subs = Effect.runSync(Ref.get(subsRef));
          for (const [, entry] of subs) {
            if (entry.serverStreamId === json.stream_id && !entry.completed) {
              Effect.runSync(Queue.offer(entry.queue, json));

              // Check if this is a FlowFinish event
              if (json.event === "FlowFinish") {
                entry.completed = true;
              }
              break;
            }
          }
        }
      } catch {
        // Ignore malformed messages
      }
    };

    // --- Close Handler ---

    const handleClose = () => {
      Effect.runSync(
        Effect.gen(function* () {
          yield* Ref.set(connRef, null);
          const intentional = yield* Ref.get(intentionalCloseRef);

          if (intentional) {
            yield* setState("disconnected");
            // Reject all pending requests
            const pending = yield* Ref.get(pendingRef);
            for (const [, p] of pending) {
              p.reject(new Error("WebSocket closed"));
            }
            yield* Ref.set(pendingRef, new Map());
          } else {
            yield* attemptReconnect();
          }
        }),
      );
    };

    // --- Reconnection ---

    const attemptReconnect = (): Effect.Effect<void> =>
      Effect.gen(function* () {
        yield* setState("waiting_to_reconnect");

        const base = Schedule.exponential(
          Duration.millis(opts.baseDelay),
        ).pipe(Schedule.jittered);

        const capped = Schedule.capDelay(
          Duration.millis(opts.maxDelay),
        )(base);

        const schedule = opts.maxRetries > 0
          ? Schedule.intersect(capped, Schedule.recurs(opts.maxRetries))
          : capped;

        yield* doConnect().pipe(
          Effect.retry(schedule),
          Effect.matchEffect({
            onFailure: () =>
              Effect.gen(function* () {
                yield* setState("disconnected");
                // Reject all pending requests
                const pending = yield* Ref.get(pendingRef);
                for (const [, p] of pending) {
                  p.reject(new Error("reconnection failed"));
                }
                yield* Ref.set(pendingRef, new Map());
              }),
            onSuccess: () => reAuth(),
          }),
        );
      }).pipe(Effect.forkDaemon, Effect.asVoid);

    // --- Re-authenticate + Re-subscribe after reconnect ---

    const reAuth = (): Effect.Effect<void> =>
      Effect.gen(function* () {
        yield* setState("authenticating");
        const token = Redacted.value(config.token);
        const id = yield* nextId();
        const msg = makeAuthenticateRequest(id, token);

        yield* send(msg, id).pipe(
          Effect.catchAll(() => Effect.void),
        );

        yield* setState("resubscribing");

        // Re-subscribe all active subscriptions
        const subs = yield* Ref.get(subsRef);
        for (const [, entry] of subs) {
          if (entry.completed) continue;

          const subId = yield* nextId();
          let subMsg;
          if (entry.type === "FlowRunEvents") {
            subMsg = makeSubscribeFlowRunEventsRequest(
              subId,
              entry.flowRunId!,
              entry.token,
            );
          } else {
            subMsg = makeSubscribeSignatureRequestsRequest(subId);
          }

          const result = yield* send<{ stream_id: number }>(subMsg, subId).pipe(
            Effect.catchAll(() => Effect.succeed(null)),
          );

          if (result !== null) {
            entry.serverStreamId = result.stream_id;
          }
        }

        yield* setState("connected");
      });

    // --- Public API ---

    return {
      connect: () =>
        Effect.gen(function* () {
          yield* doConnect();
          yield* setState("connected");
        }),

      authenticate: () =>
        Effect.gen(function* () {
          yield* setState("authenticating");
          const token = Redacted.value(config.token);
          const id = yield* nextId();
          const msg = makeAuthenticateRequest(id, token);

          const result = yield* send<AuthenticateResponseOk>(msg, id).pipe(
            Effect.mapError((e) => {
              if (e._tag === "WsTimeoutError") return e;
              return new WsProtocolError({
                method: "Authenticate",
                message: e.message,
              });
            }),
          );

          yield* setState("connected");
          return result;
        }),

      subscribeFlowRunEvents: (flowRunId, token) =>
        Stream.unwrapScoped(
          Effect.gen(function* () {
            const queue = yield* Queue.unbounded<FlowRunEvent>();
            const clientSubId = yield* Ref.getAndUpdate(
              nextSubIdRef,
              (n) => n + 1,
            );

            // Register subscription
            const entry: SubscriptionEntry = {
              clientSubId,
              serverStreamId: null,
              type: "FlowRunEvents",
              flowRunId,
              token,
              queue: queue as Queue.Queue<unknown>,
              completed: false,
            };

            yield* Ref.update(subsRef, (m) => {
              const next = new Map(m);
              next.set(clientSubId, entry);
              return next;
            });

            // Send subscribe request
            const id = yield* nextId();
            const msg = makeSubscribeFlowRunEventsRequest(id, flowRunId, token);
            const result = yield* send<{ stream_id: number }>(msg, id).pipe(
              Effect.mapError((e) =>
                new WsProtocolError({
                  method: "SubscribeFlowRunEvents",
                  message: e.message,
                })
              ),
            );
            entry.serverStreamId = result.stream_id;

            // Cleanup on scope finalization
            yield* Effect.addFinalizer(() =>
              Ref.update(subsRef, (m) => {
                const next = new Map(m);
                next.delete(clientSubId);
                return next;
              }).pipe(Effect.andThen(Queue.shutdown(queue)))
            );

            return Stream.fromQueue(queue).pipe(
              Stream.takeUntil((ev) => ev.event === "FlowFinish"),
            );
          }),
        ),

      subscribeSignatureRequests: () =>
        Stream.unwrapScoped(
          Effect.gen(function* () {
            const queue = yield* Queue.unbounded<SignatureRequestsEvent>();
            const clientSubId = yield* Ref.getAndUpdate(
              nextSubIdRef,
              (n) => n + 1,
            );

            const entry: SubscriptionEntry = {
              clientSubId,
              serverStreamId: null,
              type: "SignatureRequests",
              queue: queue as Queue.Queue<unknown>,
              completed: false,
            };

            yield* Ref.update(subsRef, (m) => {
              const next = new Map(m);
              next.set(clientSubId, entry);
              return next;
            });

            const id = yield* nextId();
            const msg = makeSubscribeSignatureRequestsRequest(id);
            const result = yield* send<{ stream_id: number }>(msg, id).pipe(
              Effect.mapError((e) =>
                new WsProtocolError({
                  method: "SubscribeSignatureRequests",
                  message: e.message,
                })
              ),
            );
            entry.serverStreamId = result.stream_id;

            yield* Effect.addFinalizer(() =>
              Ref.update(subsRef, (m) => {
                const next = new Map(m);
                next.delete(clientSubId);
                return next;
              }).pipe(Effect.andThen(Queue.shutdown(queue)))
            );

            return Stream.fromQueue(queue);
          }),
        ),

      close: () =>
        Effect.gen(function* () {
          yield* Ref.set(intentionalCloseRef, true);
          const ws = yield* Ref.get(connRef);
          if (ws !== null) {
            ws.close();
          }
          yield* Ref.set(connRef, null);
          yield* setState("disconnected");
        }),

      state: Ref.get(stateRef),
    };
  }),
);
