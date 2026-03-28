import type { ClientCore } from "../internal/core.ts";
import {
  apiKeyInfoOutputSchema,
  createApiKeyOutputSchema,
} from "@space-operator/contracts";
import type {
  ApiKeyInfoOutput,
  CreateApiKeyOutput,
  RequestOptions,
} from "../types.ts";

export function createApiKeysNamespace(core: ClientCore) {
  return {
    async create(
      name: string,
      options: RequestOptions = {},
    ): Promise<CreateApiKeyOutput> {
      return await core.requestContract(createApiKeyOutputSchema, {
        method: "POST",
        path: "/apikey/create",
        auth: options.auth,
        body: { name },
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "create api key response");
    },

    async delete(
      key_hash: string,
      options: RequestOptions = {},
    ): Promise<Record<string, never>> {
      return await core.requestJson<Record<string, never>>({
        method: "POST",
        path: "/apikey/delete",
        auth: options.auth,
        body: { key_hash },
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      });
    },

    async info(options: RequestOptions = {}): Promise<ApiKeyInfoOutput> {
      return await core.requestContract(apiKeyInfoOutputSchema, {
        method: "GET",
        path: "/apikey/info",
        auth: options.auth,
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "api key info response");
    },
  };
}

export type ApiKeysNamespace = ReturnType<typeof createApiKeysNamespace>;
