import { type IValue, Value } from "../../deps.ts";
import {
  flowRunWireEventSchemas,
  signatureRequestsEventSchema,
  webSocketIdentitySchema,
  z,
} from "@space-operator/contracts";
import type {
  AuthStrategy,
  FlowError,
  FlowRunEvent,
  FlowRunId,
  ISignatureRequest,
  SignatureRequestsEvent,
  SubscribeFlowRunOptions,
  WebSocketIdentity,
  WebSocketLike,
  WsResponse,
} from "../../types.ts";
import { SignatureRequest as SignatureRequestModel } from "../../types.ts";
import {
  getWebSocketFactory,
  log,
  type ResolvedClientConfig,
  resolveProvider,
  resolveWsToken,
  toWsUrl,
} from "../runtime.ts";
import { Effect, runClientEffect } from "../effect.ts";
import { parseContract } from "../contracts.ts";
import { withSpan } from "../telemetry.ts";
import { TransportError, WebSocketProtocolError } from "./errors.ts";

interface PendingRequest<T> {
  method?: string;
  resolve: (value: T) => void;
  reject: (error: Error) => void;
}

interface StreamHandler {
  push: (message: Record<string, unknown>) => void;
  fail: (error: Error) => void;
  finish: () => void;
}

const wsStreamStartSchema = z.object({ stream_id: z.number() }).strict();

function parseWireFlowRunEvent(message: Record<string, unknown>): FlowRunEvent {
  const stream_id = Number(message.stream_id);
  const event = String(message.event);

  switch (event) {
    case "SignatureRequest": {
      const parsed = parseContract(
        flowRunWireEventSchemas.SignatureRequest,
        message,
        "flow-run websocket event",
      );
      return {
        stream_id,
        event,
        data: new SignatureRequestModel(parsed.data as ISignatureRequest),
      } as FlowRunEvent;
    }
    case "FlowFinish": {
      const parsed = parseContract(
        flowRunWireEventSchemas.FlowFinish,
        message,
        "flow-run websocket event",
      );
      return {
        stream_id,
        event,
        data: {
          ...(parsed.data as Record<string, unknown>),
          output: Value.fromJSON(parsed.data.output as IValue),
        } as FlowRunEvent["data"],
      } as FlowRunEvent;
    }
    case "NodeStart": {
      const parsed = parseContract(
        flowRunWireEventSchemas.NodeStart,
        message,
        "flow-run websocket event",
      );
      return {
        stream_id,
        event,
        data: {
          ...(parsed.data as Record<string, unknown>),
          input: Value.fromJSON(parsed.data.input as IValue),
        } as FlowRunEvent["data"],
      } as FlowRunEvent;
    }
    case "NodeOutput": {
      const parsed = parseContract(
        flowRunWireEventSchemas.NodeOutput,
        message,
        "flow-run websocket event",
      );
      return {
        stream_id,
        event,
        data: {
          ...(parsed.data as Record<string, unknown>),
          output: Value.fromJSON(parsed.data.output as IValue),
        } as FlowRunEvent["data"],
      } as FlowRunEvent;
    }
    case "FlowError": {
      const parsed = parseContract(
        flowRunWireEventSchemas.FlowError,
        message,
        "flow-run websocket event",
      );
      return {
        stream_id,
        event,
        data: parsed.data as FlowError,
      } as FlowRunEvent;
    }
    default:
      return {
        stream_id,
        event: event as FlowRunEvent["event"],
        data: (message.data ?? {}) as FlowRunEvent["data"],
      } as FlowRunEvent;
  }
}

function parseWireSignatureRequestsEvent(
  message: Record<string, unknown>,
): SignatureRequestsEvent {
  const parsed = parseContract(
    signatureRequestsEventSchema,
    message,
    "signature-request websocket event",
  );
  return {
    stream_id: parsed.stream_id,
    event: "SignatureRequest",
    data: new SignatureRequestModel(parsed.data as ISignatureRequest),
  };
}

function normalizeProtocolError(error: unknown): Error {
  if (
    error instanceof WebSocketProtocolError || error instanceof TransportError
  ) {
    return error;
  }
  if (error instanceof Error) {
    return new WebSocketProtocolError(error.message, { cause: error });
  }
  if (typeof error === "string" && error.length > 0) {
    return new WebSocketProtocolError(error);
  }
  return new WebSocketProtocolError("invalid websocket payload");
}

function normalizeWsEffectError(
  error: unknown,
  fallback: string,
): Error {
  if (error instanceof Error) {
    return error;
  }
  if (typeof error === "string" && error.length > 0) {
    return new WebSocketProtocolError(error);
  }
  return new TransportError(fallback, {
    cause: new Error(String(error)),
  });
}

function promiseEffect<T>(
  operation: () => Promise<T>,
  fallback: string,
): Effect.Effect<T, Error> {
  return Effect.tryPromise({
    try: operation,
    catch: (error) => normalizeWsEffectError(error, fallback),
  });
}

function hasOwnAuth(
  options: { auth?: AuthStrategy | undefined },
): boolean {
  return Object.prototype.hasOwnProperty.call(options, "auth");
}

function resolveEffectiveAuth<T extends { auth?: AuthStrategy | undefined }>(
  defaultAuth: AuthStrategy | undefined,
  options: T,
): AuthStrategy | undefined {
  return hasOwnAuth(options) ? options.auth : defaultAuth;
}

export class EventSubscription<T>
  implements AsyncIterable<T>, AsyncIterator<T> {
  private readonly queue: T[] = [];
  private readonly pending: Array<PendingRequest<IteratorResult<T>>> = [];
  private readonly resolveClosed: () => void;
  private done = false;
  private failure?: Error;
  readonly closed: Promise<void>;

  constructor(private readonly closeImpl: () => void) {
    let resolveClosed!: () => void;
    this.closed = new Promise((resolve) => {
      resolveClosed = resolve;
    });
    this.resolveClosed = resolveClosed;
  }

  [Symbol.asyncIterator](): AsyncIterator<T> {
    return this;
  }

  next(): Promise<IteratorResult<T>> {
    if (this.queue.length > 0) {
      return Promise.resolve({ value: this.queue.shift()!, done: false });
    }
    if (this.failure) {
      return Promise.reject(this.failure);
    }
    if (this.done) {
      return Promise.resolve({ value: undefined, done: true });
    }

    return new Promise((resolve, reject) => {
      this.pending.push({ resolve, reject });
    });
  }

  async close(): Promise<void> {
    if (this.done) {
      return await this.closed;
    }
    this.closeImpl();
    this.finish();
    return await this.closed;
  }

  push(value: T) {
    if (this.done) {
      return;
    }
    const waiting = this.pending.shift();
    if (waiting) {
      waiting.resolve({ value, done: false });
      return;
    }
    this.queue.push(value);
  }

  fail(error: Error) {
    if (this.done) {
      return;
    }
    this.done = true;
    this.failure = error;
    while (this.pending.length > 0) {
      this.pending.shift()!.reject(error);
    }
    this.resolveClosed();
  }

  finish() {
    if (this.done) {
      return;
    }
    this.done = true;
    while (this.pending.length > 0) {
      this.pending.shift()!.resolve({ value: undefined, done: true });
    }
    this.resolveClosed();
  }
}

class RpcSocket {
  private readonly pending = new Map<number, PendingRequest<WsResponse<any>>>();
  private readonly socket: WebSocketLike;
  private nextRequestId = 0;
  private opened = false;
  private readonly openPromise: Promise<void>;
  private readonly resolveOpen: () => void;
  private readonly rejectOpen: (error: Error) => void;
  private readonly closedPromise: Promise<void>;
  private readonly resolveClosed: () => void;
  private closeRequested = false;
  private failure?: Error;
  private readonly streams = new Map<number, StreamHandler>();
  private readonly futureStreams = new Map<number, Record<string, unknown>[]>();
  private readonly closedStreams = new Set<number>();

  constructor(private readonly config: ResolvedClientConfig) {
    let resolveOpen!: () => void;
    let rejectOpen!: (error: Error) => void;
    let resolveClosed!: () => void;
    this.openPromise = new Promise((resolve, reject) => {
      resolveOpen = resolve;
      rejectOpen = reject;
    });
    this.closedPromise = new Promise((resolve) => {
      resolveClosed = resolve;
    });
    this.resolveOpen = resolveOpen;
    this.rejectOpen = rejectOpen;
    this.resolveClosed = resolveClosed;
    this.socket = getWebSocketFactory(config)(toWsUrl(config.baseUrl));
    this.socket.onopen = () => {
      this.opened = true;
      this.resolveOpen();
    };
    this.socket.onmessage = (event) => this.onMessage(event);
    this.socket.onerror = () => {
      const error = new TransportError("websocket error");
      this.rejectAll(error);
      if (!this.opened) {
        this.rejectOpen(error);
      }
    };
    this.socket.onclose = (event) => {
      const error = this.closeRequested ? undefined : new TransportError(
        `websocket closed${event.reason ? `: ${event.reason}` : ""}`,
      );
      if (error) {
        this.rejectAll(error);
        if (!this.opened) {
          this.rejectOpen(error);
        }
      } else {
        this.finishAll();
      }
      this.resolveClosed();
    };
  }

  private waitUntilOpenEffect(): Effect.Effect<void, Error> {
    return promiseEffect(
      () => this.openPromise,
      "failed while opening websocket",
    );
  }

  async waitUntilOpen(): Promise<void> {
    await runClientEffect(this.waitUntilOpenEffect());
  }

  requestEffect<T>(
    method: string,
    params: Record<string, unknown>,
  ): Effect.Effect<WsResponse<T>, Error> {
    const self = this;
    return Effect.gen(function* () {
      yield* self.waitUntilOpenEffect();
      const id = ++self.nextRequestId;
      const payload = JSON.stringify({ id, method, params });

      const result = new Promise<WsResponse<T>>((resolve, reject) => {
        self.pending.set(id, {
          method,
          resolve: resolve as PendingRequest<WsResponse<any>>["resolve"],
          reject,
        });
      });

      log(self.config, {
        scope: "ws",
        event: "send",
        data: { id, method, params },
      });
      yield* Effect.try({
        try: () => {
          self.socket.send(payload);
        },
        catch: (error) => {
          self.pending.delete(id);
          return normalizeProtocolError(error);
        },
      });
      return yield* promiseEffect(
        () => result,
        `websocket request ${method} failed`,
      );
    }) as Effect.Effect<WsResponse<T>, Error>;
  }

  async request<T>(
    method: string,
    params: Record<string, unknown>,
  ): Promise<WsResponse<T>> {
    return await runClientEffect(this.requestEffect(method, params));
  }

  subscribe<T>(
    streamId: number,
    parser: (message: Record<string, unknown>) => T,
  ): EventSubscription<T> {
    const subscription = new EventSubscription<T>(() => {
      this.streams.delete(streamId);
      this.futureStreams.delete(streamId);
      this.closedStreams.add(streamId);
    });
    if (this.failure) {
      subscription.fail(this.failure);
      return subscription;
    }
    this.closedStreams.delete(streamId);
    const push = (message: Record<string, unknown>) => {
      try {
        subscription.push(parser(message));
      } catch (error) {
        this.handleFailure(normalizeProtocolError(error));
      }
    };
    this.streams.set(streamId, {
      push,
      fail: (error) => subscription.fail(error),
      finish: () => subscription.finish(),
    });
    const queued = this.futureStreams.get(streamId);
    if (queued) {
      for (const message of queued) {
        push(message);
      }
      this.futureStreams.delete(streamId);
    }
    return subscription;
  }

  closeEffect(reason = "session closed"): Effect.Effect<void, Error> {
    const self = this;
    return Effect.gen(function* () {
      if (!self.closeRequested) {
        self.closeRequested = true;
        try {
          self.socket.close(1000, reason);
        } catch {
          self.finishAll();
          self.resolveClosed();
        }
      }
      yield* promiseEffect(
        () => self.closedPromise,
        "failed while closing websocket",
      );
    }) as Effect.Effect<void, Error>;
  }

  async close(reason = "session closed"): Promise<void> {
    await runClientEffect(this.closeEffect(reason));
  }

  private onMessage(event: { data: unknown }) {
    if (typeof event.data !== "string") {
      this.handleFailure(
        new WebSocketProtocolError("invalid websocket payload"),
      );
      return;
    }
    try {
      const json = JSON.parse(event.data) as Record<string, unknown>;
      log(this.config, {
        scope: "ws",
        event: "receive",
        data: json,
      });
      if (typeof json.id === "number") {
        const pending = this.pending.get(json.id);
        if (!pending) {
          this.handleFailure(
            new WebSocketProtocolError(
              `missing request handler for ${json.id}`,
            ),
          );
          return;
        }
        this.pending.delete(json.id);
        if (
          (pending.method === "SubscribeFlowRunEvents" ||
            pending.method === "SubscribeSignatureRequests") &&
          typeof (json.Ok as { stream_id?: unknown } | undefined)?.stream_id ===
            "number"
        ) {
          this.closedStreams.delete(
            Number((json.Ok as { stream_id: number }).stream_id),
          );
        }
        pending.resolve(json as unknown as WsResponse<any>);
        return;
      }
      if (typeof json.stream_id === "number") {
        const streamId = Number(json.stream_id);
        if (this.closedStreams.has(streamId)) {
          return;
        }
        const handler = this.streams.get(streamId);
        if (handler) {
          handler.push(json);
        } else {
          const queued = this.futureStreams.get(streamId) ?? [];
          queued.push(json);
          this.futureStreams.set(streamId, queued);
        }
        return;
      }
      this.handleFailure(
        new WebSocketProtocolError("invalid websocket payload"),
      );
    } catch (error) {
      this.handleFailure(normalizeProtocolError(error));
    }
  }

  private rejectAll(error: Error) {
    this.failure = error;
    for (const pending of this.pending.values()) {
      pending.reject(error);
    }
    this.pending.clear();
    for (const stream of this.streams.values()) {
      stream.fail(error);
    }
    this.streams.clear();
    this.futureStreams.clear();
    this.closedStreams.clear();
  }

  private finishAll() {
    for (const stream of this.streams.values()) {
      stream.finish();
    }
    this.streams.clear();
    this.futureStreams.clear();
    this.closedStreams.clear();
  }

  private handleFailure(error: Error) {
    this.rejectAll(error);
    if (!this.opened) {
      this.rejectOpen(error);
    }
    if (!this.closeRequested) {
      this.closeRequested = true;
      try {
        this.socket.close(1002, error.message);
      } catch {
        // Ignore close failures while surfacing the original error.
      }
    }
    this.resolveClosed();
  }
}

function bindAbort<T>(
  subscription: EventSubscription<T>,
  signal?: AbortSignal,
) {
  signal?.addEventListener("abort", () => {
    subscription.close().catch(() => undefined);
  }, { once: true });
}

export class WebSocketSession {
  private rpc: RpcSocket;
  private authenticatedToken?: string | null;
  private identity?: WebSocketIdentity;

  constructor(
    private readonly config: ResolvedClientConfig,
    private readonly auth?: AuthStrategy,
  ) {
    this.rpc = new RpcSocket(config);
  }

  private resetConnectionEffect(reason: string): Effect.Effect<void, Error> {
    const self = this;
    return Effect.gen(function* () {
      yield* self.rpc.closeEffect(reason);
      self.rpc = new RpcSocket(self.config);
      self.identity = undefined;
      self.authenticatedToken = undefined;
    }) as Effect.Effect<void, Error>;
  }

  getIdentity(): WebSocketIdentity | undefined {
    return this.identity;
  }

  async authenticate(
    ...args: [] | [AuthStrategy | undefined]
  ): Promise<WebSocketIdentity | undefined> {
    const auth = args.length === 0 ? this.auth ?? this.config.auth : args[0];
    return await withSpan(
      this.config.telemetry,
      "space_operator.ws.authenticate",
      {
        "space_operator.auth.kind": auth?.kind ?? "none",
      },
      async () =>
        parseContract(
          webSocketIdentitySchema.optional(),
          await runClientEffect(this.authenticateEffect(auth)),
          "websocket identity",
        ),
    );
  }

  private authenticateEffect(
    auth: AuthStrategy | undefined,
  ): Effect.Effect<WebSocketIdentity | undefined, Error> {
    const self = this;
    return Effect.gen(function* () {
      const nextToken = (yield* promiseEffect(
        () => resolveWsToken(auth),
        "failed to resolve websocket auth",
      )) ?? null;
      if (
        self.authenticatedToken !== undefined &&
        self.authenticatedToken !== nextToken
      ) {
        yield* self.resetConnectionEffect("authentication changed");
      }
      if (nextToken === null) {
        self.authenticatedToken = null;
        self.identity = undefined;
        return undefined;
      }
      if (self.authenticatedToken === nextToken) {
        return self.identity;
      }
      self.identity = ensureWsSuccess<WebSocketIdentity>(
        yield* self.rpc.requestEffect<WebSocketIdentity>("Authenticate", {
          token: nextToken,
        }),
      );
      self.authenticatedToken = nextToken;
      return self.identity;
    }) as Effect.Effect<WebSocketIdentity | undefined, Error>;
  }

  async subscribeFlowRun(
    flowRunId: FlowRunId,
    options: SubscribeFlowRunOptions = {},
  ): Promise<EventSubscription<FlowRunEvent>> {
    const effectiveAuth = resolveEffectiveAuth(
      this.auth ?? this.config.auth,
      options,
    );
    return await withSpan(
      this.config.telemetry,
      "space_operator.ws.subscribe_flow_run",
      {
        "space_operator.flow_run.id": flowRunId,
        "space_operator.auth.kind": effectiveAuth?.kind ?? "none",
      },
      async () =>
        await runClientEffect(
          this.subscribeFlowRunEffect(flowRunId, options, effectiveAuth),
        ),
    );
  }

  private subscribeFlowRunEffect(
    flowRunId: FlowRunId,
    options: SubscribeFlowRunOptions,
    effectiveAuth: AuthStrategy | undefined,
  ): Effect.Effect<EventSubscription<FlowRunEvent>, Error> {
    const self = this;
    return Effect.gen(function* () {
      yield* self.authenticateEffect(effectiveAuth);
      const streamToken = options.token
        ? yield* promiseEffect(
          () => resolveProvider(options.token!),
          "failed to resolve flow run stream token",
        )
        : undefined;
      const stream = parseContract(
        wsStreamStartSchema,
        ensureWsSuccess(
          yield* self.rpc.requestEffect<{ stream_id: number }>(
            "SubscribeFlowRunEvents",
            {
              flow_run_id: flowRunId,
              ...(streamToken ? { token: streamToken } : {}),
            },
          ),
        ),
        "websocket stream start",
      );
      const subscription = self.rpc.subscribe(
        stream.stream_id,
        parseWireFlowRunEvent,
      );
      bindAbort(subscription, options.signal);
      return subscription;
    }) as Effect.Effect<EventSubscription<FlowRunEvent>, Error>;
  }

  async subscribeSignatureRequests(
    options: { auth?: AuthStrategy; signal?: AbortSignal } = {},
  ): Promise<EventSubscription<SignatureRequestsEvent>> {
    const effectiveAuth = resolveEffectiveAuth(
      this.auth ?? this.config.auth,
      options,
    );
    return await withSpan(
      this.config.telemetry,
      "space_operator.ws.subscribe_signature_requests",
      {
        "space_operator.auth.kind": effectiveAuth?.kind ?? "none",
      },
      async () =>
        await runClientEffect(
          this.subscribeSignatureRequestsEffect(options, effectiveAuth),
        ),
    );
  }

  private subscribeSignatureRequestsEffect(
    options: { auth?: AuthStrategy; signal?: AbortSignal },
    effectiveAuth: AuthStrategy | undefined,
  ): Effect.Effect<EventSubscription<SignatureRequestsEvent>, Error> {
    const self = this;
    return Effect.gen(function* () {
      const wsToken = yield* promiseEffect(
        () => resolveWsToken(effectiveAuth),
        "failed to resolve websocket auth",
      );
      if (!wsToken) {
        return yield* Effect.fail(
          new WebSocketProtocolError(
            "signature request subscriptions require apiKey or bearer auth",
          ),
        );
      }
      yield* self.authenticateEffect(effectiveAuth);
      const stream = parseContract(
        wsStreamStartSchema,
        ensureWsSuccess(
          yield* self.rpc.requestEffect<{ stream_id: number }>(
            "SubscribeSignatureRequests",
            {},
          ),
        ),
        "websocket stream start",
      );
      const subscription = self.rpc.subscribe(
        stream.stream_id,
        parseWireSignatureRequestsEvent,
      );
      bindAbort(subscription, options.signal);
      return subscription;
    }) as Effect.Effect<EventSubscription<SignatureRequestsEvent>, Error>;
  }

  async close(): Promise<void> {
    await withSpan(
      this.config.telemetry,
      "space_operator.ws.close",
      {},
      async () => {
        await runClientEffect(this.rpc.closeEffect());
      },
    );
  }
}

function ensureWsSuccess<T>(response: WsResponse<T>): T {
  if (response.Err !== undefined) {
    throw new WebSocketProtocolError(response.Err);
  }
  if (response.Ok === undefined) {
    throw new WebSocketProtocolError("websocket response missing Ok payload");
  }
  return response.Ok;
}

export async function subscribeFlowRun(
  config: ResolvedClientConfig,
  flowRunId: FlowRunId,
  options: SubscribeFlowRunOptions = {},
): Promise<EventSubscription<FlowRunEvent>> {
  const session = new WebSocketSession(
    config,
    resolveEffectiveAuth(config.auth, options),
  );
  try {
    const subscription = await session.subscribeFlowRun(flowRunId, options);
    void subscription.closed.finally(() =>
      session.close().catch(() => undefined)
    );
    return subscription;
  } catch (error) {
    await session.close().catch(() => undefined);
    throw error;
  }
}

export async function subscribeSignatureRequests(
  config: ResolvedClientConfig,
  options: { auth?: AuthStrategy; signal?: AbortSignal } = {},
): Promise<EventSubscription<SignatureRequestsEvent>> {
  const session = new WebSocketSession(
    config,
    resolveEffectiveAuth(config.auth, options),
  );
  try {
    const subscription = await session.subscribeSignatureRequests(options);
    void subscription.closed.finally(() =>
      session.close().catch(() => undefined)
    );
    return subscription;
  } catch (error) {
    await session.close().catch(() => undefined);
    throw error;
  }
}
