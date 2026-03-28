import type { ClientCore } from "../internal/core.ts";
import {
  EventSubscription,
  WebSocketSession,
} from "../internal/transport/ws.ts";
import type {
  AuthStrategy,
  FlowRunEvent,
  FlowRunId,
  SignatureRequestsEvent,
  SubscribeFlowRunOptions,
} from "../types.ts";

function resolveEventAuth(
  defaultAuth: AuthStrategy | undefined,
  options: { auth?: AuthStrategy | undefined },
): AuthStrategy | undefined {
  return Object.prototype.hasOwnProperty.call(options, "auth")
    ? options.auth
    : defaultAuth;
}

export function createEventsNamespace(core: ClientCore) {
  return {
    session(
      options: { auth?: AuthStrategy } = {},
    ): WebSocketSession {
      return new WebSocketSession(
        core.config,
        resolveEventAuth(core.config.auth, options),
      );
    },

    async subscribeFlowRun(
      flowRunId: FlowRunId,
      options: SubscribeFlowRunOptions = {},
    ): Promise<EventSubscription<FlowRunEvent>> {
      return await core.subscribeFlowRun(flowRunId, options);
    },

    async subscribeSignatureRequests(
      options: { auth?: AuthStrategy; signal?: AbortSignal } = {},
    ): Promise<EventSubscription<SignatureRequestsEvent>> {
      return await core.subscribeSignatureRequests(options);
    },
  };
}

export type EventsNamespace = ReturnType<typeof createEventsNamespace>;
export { EventSubscription, WebSocketSession };
