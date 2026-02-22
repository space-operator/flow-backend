import { Duration, Effect, Schedule, Stream } from "effect";
import type { Value } from "./deps.ts";
import type { FlowId, FlowRunId } from "./Schema/Common.ts";
import type { StartFlowParams } from "./Schema/Rest.ts";
import type { FlowRunEvent } from "./Schema/Ws.ts";
import {
  AuthTokenError,
  HttpApiError,
  type WsConnectionError,
  type WsProtocolError,
  type WsTimeoutError,
} from "./Errors.ts";
import { FlowService } from "./FlowService.ts";
import { WsService } from "./WsService.ts";

// --- Options ---

export interface RunFlowOptions {
  /** Initial polling interval in ms. Default: 1000. */
  baseDelay?: number;
  /** Maximum polling interval cap in ms. Default: 5000. */
  maxDelay?: number;
  /** Maximum total wait time in ms. Default: 300_000 (5 min). */
  timeout?: number;
}

export interface RunFlowWsOptions {
  /** Called for each FlowRunEvent as it arrives. */
  onEvent?: (event: FlowRunEvent) => void;
  /** Collect all events and return them in the result. Default: false. */
  collectEvents?: boolean;
  /** Maximum total wait time in ms. Default: 300_000 (5 min). */
  timeout?: number;
}

export interface RunFlowWsResult {
  output: Value;
  events: FlowRunEvent[];
}

// --- runFlow (HTTP polling, no WS) ---

/**
 * Start a flow and poll for its output.
 *
 * Calls `startFlow`, then retries `getFlowOutput` with exponential backoff
 * until the output is available or the timeout is reached.
 *
 * For shared/unverified flows, use `startFlowShared`/`startFlowUnverified`
 * directly and poll with `getFlowOutput` + `Effect.retry`.
 */
export const runFlow = (
  id: FlowId | string,
  params: StartFlowParams,
  opts?: RunFlowOptions,
): Effect.Effect<Value, HttpApiError | AuthTokenError, FlowService> =>
  Effect.gen(function* () {
    const flow = yield* FlowService;

    const baseDelay = opts?.baseDelay ?? 1000;
    const maxDelay = opts?.maxDelay ?? 5000;
    const timeout = opts?.timeout ?? 300_000;

    const { flow_run_id } = yield* flow.startFlow(id, params);

    // Exponential backoff with jitter, capped delay, only retry retriable errors
    const schedule = Schedule.exponential(Duration.millis(baseDelay)).pipe(
      Schedule.jittered,
      (s) => Schedule.capDelay(Duration.millis(maxDelay))(s),
      Schedule.whileInput<HttpApiError | AuthTokenError>((err) => {
        // Don't retry auth errors — they're permanent
        if (err._tag === "AuthTokenError") return false;
        if (err._tag === "HttpApiError") {
          return err.status !== 401 && err.status !== 403;
        }
        return false;
      }),
    );

    return yield* flow.getFlowOutput(flow_run_id).pipe(
      Effect.retry(schedule),
      Effect.timeoutFail({
        duration: Duration.millis(timeout),
        onTimeout: () =>
          new HttpApiError({
            status: 0,
            url: "",
            body: "",
            message: `runFlow timed out after ${timeout}ms waiting for output`,
          }),
      }),
    );
  });

// --- runFlowWs (WebSocket-based) ---

/**
 * Start a flow, subscribe to its events via WebSocket, and return the output.
 *
 * Connects and authenticates the WS if not already connected. Subscribes to
 * flow run events, waits for FlowFinish, then fetches the canonical output
 * via HTTP.
 */
export const runFlowWs = (
  id: FlowId | string,
  params: StartFlowParams,
  opts?: RunFlowWsOptions,
): Effect.Effect<
  RunFlowWsResult,
  | HttpApiError
  | AuthTokenError
  | WsProtocolError
  | WsConnectionError
  | WsTimeoutError,
  FlowService | WsService
> =>
  Effect.scoped(
    Effect.gen(function* () {
      const flow = yield* FlowService;
      const ws = yield* WsService;

      const timeout = opts?.timeout ?? 300_000;

      // 1. Start the flow
      const { flow_run_id } = yield* flow.startFlow(id, params);

      // 2. Connect + authenticate if not already connected
      const currentState = yield* ws.state;
      if (currentState === "disconnected") {
        yield* ws.connect();
        yield* ws.authenticate();
      }

      // 3. Subscribe to events
      const eventStream = ws.subscribeFlowRunEvents(flow_run_id);

      // 4. Consume stream, collecting events if requested
      const collectedEvents: FlowRunEvent[] = [];

      yield* eventStream.pipe(
        Stream.runForEach((event) =>
          Effect.sync(() => {
            if (opts?.onEvent) opts.onEvent(event);
            if (opts?.collectEvents) collectedEvents.push(event);
          })
        ),
        Effect.timeoutFail({
          duration: Duration.millis(timeout),
          onTimeout: () =>
            new HttpApiError({
              status: 0,
              url: "",
              body: "",
              message: `runFlowWs timed out after ${timeout}ms waiting for FlowFinish`,
            }),
        }),
      );

      // 5. Stream ended (FlowFinish received) — fetch canonical output via HTTP
      const output = yield* flow.getFlowOutput(flow_run_id);

      return { output, events: collectedEvents } as RunFlowWsResult;
    }),
  );
