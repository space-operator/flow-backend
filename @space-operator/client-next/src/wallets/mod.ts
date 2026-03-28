import type { ClientCore } from "../internal/core.ts";
import type { RequestOptions, WalletUpsertBody } from "../types.ts";

export function createWalletsNamespace(core: ClientCore) {
  return {
    async upsert<TResult = unknown>(
      body: WalletUpsertBody,
      options: RequestOptions = {},
    ): Promise<TResult> {
      return await core.requestJson<TResult>({
        method: "POST",
        path: "/wallets/upsert",
        auth: options.auth,
        body,
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      });
    },
  };
}

export type WalletsNamespace = ReturnType<typeof createWalletsNamespace>;
