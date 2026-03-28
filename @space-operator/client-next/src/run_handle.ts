import { type IValue, Value } from "./deps.ts";
import {
  iValueSchema,
  signatureRequestSchema,
  successResponseSchema,
} from "@space-operator/contracts";
import { flowRunTokenAuth } from "./auth/mod.ts";
import type { ClientCore } from "./internal/core.ts";
import { FlowRunFailedError } from "./internal/transport/errors.ts";
import type {
  AuthStrategy,
  FlowFinish,
  FlowRunEvent,
  FlowRunId,
  ISignatureRequest,
  RequestOptions,
  SignatureRequest,
  StopFlowParams,
  SubscribeFlowRunOptions,
  SuccessResponse,
} from "./types.ts";
import { SignatureRequest as SignatureRequestModel } from "./types.ts";
import { EventSubscription } from "./internal/transport/ws.ts";

function resolveHandleAuth(
  auth?: AuthStrategy,
  token?: string,
): AuthStrategy | undefined {
  if (auth !== undefined) {
    return auth;
  }
  if (token !== undefined) {
    return flowRunTokenAuth(token);
  }
  return undefined;
}

export class FlowRunHandle {
  constructor(
    private readonly core: ClientCore,
    readonly id: FlowRunId,
    readonly token?: string,
    private readonly authOverride?: AuthStrategy,
  ) {}

  withAuth(auth: AuthStrategy): FlowRunHandle {
    return new FlowRunHandle(this.core, this.id, this.token, auth);
  }

  private authFor(options?: { auth?: AuthStrategy }): AuthStrategy | undefined {
    return resolveHandleAuth(options?.auth ?? this.authOverride, this.token);
  }

  async output(options: RequestOptions = {}): Promise<Value> {
    const value = await this.core.requestContract(iValueSchema, {
      method: "GET",
      path: `/flow/output/${this.id}`,
      auth: this.authFor(options),
      headers: options.headers,
      signal: options.signal,
      retry: options.retry,
      timeoutMs: options.timeoutMs,
    }, "flow output response");
    return Value.fromJSON(value as IValue);
  }

  async stop(
    params: StopFlowParams = {},
    options: RequestOptions = {},
  ): Promise<SuccessResponse> {
    return await this.core.requestContract(successResponseSchema, {
      method: "POST",
      path: `/flow/stop/${this.id}`,
      auth: this.authFor(options),
      body: params,
      headers: options.headers,
      signal: options.signal,
      retry: options.retry,
      timeoutMs: options.timeoutMs,
    }, "flow stop response");
  }

  async signatureRequest(
    options: RequestOptions = {},
  ): Promise<SignatureRequest> {
    const value = await this.core.requestContract(signatureRequestSchema, {
      method: "GET",
      path: `/flow/signature_request/${this.id}`,
      auth: this.authFor(options),
      headers: options.headers,
      signal: options.signal,
      retry: options.retry,
      timeoutMs: options.timeoutMs,
    }, "signature request response");
    return new SignatureRequestModel(value as ISignatureRequest);
  }

  async events(
    options: SubscribeFlowRunOptions = {},
  ): Promise<EventSubscription<FlowRunEvent>> {
    return await this.core.subscribeFlowRun(this.id, {
      ...options,
      auth: this.authFor(options),
    });
  }

  async waitForFinish(
    options: SubscribeFlowRunOptions = {},
  ): Promise<FlowFinish> {
    const subscription = await this.events(options);
    try {
      for await (const event of subscription) {
        if (event.event === "FlowFinish") {
          return event.data;
        }
        if (event.event === "FlowError") {
          throw new FlowRunFailedError(event.data);
        }
      }
    } finally {
      await subscription.close();
    }
    throw new Error(`flow run ${this.id} finished without FlowFinish event`);
  }
}
