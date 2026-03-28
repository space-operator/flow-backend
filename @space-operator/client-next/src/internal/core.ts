import { serviceInfoOutputSchema } from "@space-operator/contracts";
import type { ZodType } from "zod";
import type {
  AuthStrategy,
  CreateClientOptions,
  ServiceInfoOutput,
} from "../types.ts";
import { type JsonRequestOptions, requestJson } from "./transport/http.ts";
import {
  EventSubscription,
  subscribeFlowRun,
  subscribeSignatureRequests,
} from "./transport/ws.ts";
import { resolveClientConfig, resolveProvider } from "./runtime.ts";
import { parseContract } from "./contracts.ts";
import type {
  FlowRunEvent,
  FlowRunId,
  SignatureRequestsEvent,
  SubscribeFlowRunOptions,
} from "../types.ts";

export class ClientCore {
  readonly config;
  private anonKeyPromise?: Promise<string>;

  constructor(private readonly options: CreateClientOptions) {
    this.config = resolveClientConfig(options);
  }

  withAuth(auth?: AuthStrategy): ClientCore {
    return new ClientCore({
      ...this.options,
      auth,
    });
  }

  async requestJson<T>(options: JsonRequestOptions): Promise<T> {
    return await requestJson<T>(this.config, options);
  }

  async requestContract<T>(
    schema: ZodType<T>,
    options: JsonRequestOptions,
    subject: string,
  ): Promise<T> {
    return parseContract(
      schema,
      await this.requestJson<unknown>(options),
      subject,
    );
  }

  async resolveAnonKey(): Promise<string> {
    if (this.options.anonKey !== undefined) {
      return await resolveProvider(this.options.anonKey);
    }

    this.anonKeyPromise ??= this.requestContract(
      serviceInfoOutputSchema,
      {
        method: "GET",
        path: "/info",
        auth: false,
      },
      "service info response",
    ).then((info) => info.anon_key);
    return await this.anonKeyPromise;
  }

  async subscribeFlowRun(
    flowRunId: FlowRunId,
    options?: SubscribeFlowRunOptions,
  ): Promise<EventSubscription<FlowRunEvent>> {
    return await subscribeFlowRun(this.config, flowRunId, options);
  }

  async subscribeSignatureRequests(
    options?: { auth?: AuthStrategy; signal?: AbortSignal },
  ): Promise<EventSubscription<SignatureRequestsEvent>> {
    return await subscribeSignatureRequests(this.config, options);
  }
}
