import type { ClientCore } from "../internal/core.ts";
import {
  serviceInfoOutputSchema,
  successResponseSchema,
} from "@space-operator/contracts";
import type {
  RequestOptions,
  ServiceInfoOutput,
  SuccessResponse,
} from "../types.ts";

export function createServiceNamespace(core: ClientCore) {
  return {
    async info(
      options: Omit<RequestOptions, "auth"> = {},
    ): Promise<ServiceInfoOutput> {
      return await core.requestContract(serviceInfoOutputSchema, {
        method: "GET",
        path: "/info",
        auth: false,
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "service info response");
    },

    async healthcheck(
      options: Omit<RequestOptions, "auth"> = {},
    ): Promise<SuccessResponse> {
      return await core.requestContract(successResponseSchema, {
        method: "GET",
        path: "/healthcheck",
        auth: false,
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "healthcheck response");
    },
  };
}

export type ServiceNamespace = ReturnType<typeof createServiceNamespace>;
