import { type IValue, Value } from "../deps.ts";
import {
  iValueSchema,
  successResponseSchema,
  z,
} from "@space-operator/contracts";
import type { ClientCore } from "../internal/core.ts";
import { normalizeFlowValue } from "../internal/transport/value.ts";
import type { RequestOptions, SuccessResponse } from "../types.ts";

export function createKvNamespace(core: ClientCore) {
  return {
    async createStore(
      store: string,
      options: RequestOptions = {},
    ): Promise<SuccessResponse> {
      return await core.requestContract(successResponseSchema, {
        method: "POST",
        path: "/kv/create_store",
        auth: options.auth,
        body: { store },
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "kv create store response");
    },

    async deleteStore(
      store: string,
      options: RequestOptions = {},
    ): Promise<SuccessResponse> {
      return await core.requestContract(successResponseSchema, {
        method: "POST",
        path: "/kv/delete_store",
        auth: options.auth,
        body: { store },
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "kv delete store response");
    },

    async write(
      store: string,
      key: string,
      value: unknown,
      options: RequestOptions = {},
    ): Promise<{ old_value?: Value }> {
      const result = await core.requestContract(
        z.object({ old_value: iValueSchema.nullish() }).passthrough(),
        {
          method: "POST",
          path: "/kv/write_item",
          auth: options.auth,
          body: {
            store,
            key,
            value: normalizeFlowValue(value),
          },
          headers: options.headers,
          signal: options.signal,
          retry: options.retry,
          timeoutMs: options.timeoutMs,
        },
        "kv write response",
      );
      return {
        old_value: result.old_value
          ? Value.fromJSON(result.old_value as IValue)
          : undefined,
      };
    },

    async read(
      store: string,
      key: string,
      options: RequestOptions = {},
    ): Promise<Value> {
      const result = await core.requestContract(
        z.object({ value: iValueSchema }).passthrough(),
        {
          method: "POST",
          path: "/kv/read_item",
          auth: options.auth,
          body: { store, key },
          headers: options.headers,
          signal: options.signal,
          retry: options.retry,
          timeoutMs: options.timeoutMs,
        },
        "kv read response",
      );
      return Value.fromJSON(result.value as IValue);
    },

    async deleteItem(
      store: string,
      key: string,
      options: RequestOptions = {},
    ): Promise<{ old_value: Value }> {
      const result = await core.requestContract(
        z.object({ old_value: iValueSchema.nullish() }).passthrough(),
        {
          method: "POST",
          path: "/kv/delete_item",
          auth: options.auth,
          body: { store, key },
          headers: options.headers,
          signal: options.signal,
          retry: options.retry,
          timeoutMs: options.timeoutMs,
        },
        "kv delete item response",
      );
      if (result.old_value == null) {
        throw new Error("kv delete item response missing old_value");
      }
      return { old_value: Value.fromJSON(result.old_value as IValue) };
    },
  };
}

export type KvNamespace = ReturnType<typeof createKvNamespace>;
