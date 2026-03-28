import type { ClientCore } from "../internal/core.ts";
import type { RequestOptions } from "../types.ts";

export function createDataNamespace(core: ClientCore) {
  return {
    async export<T = unknown>(options: RequestOptions = {}): Promise<T> {
      return await core.requestJson<T>({
        method: "POST",
        path: "/data/export",
        auth: options.auth,
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      });
    },
  };
}

export type DataNamespace = ReturnType<typeof createDataNamespace>;
