import { encodeBase58 } from "../deps.ts";
import {
  claimTokenOutputSchema,
  confirmAuthOutputSchema,
  z,
} from "@space-operator/contracts";
import type { ClientCore } from "../internal/core.ts";
import type {
  ApiKeyAuth,
  AuthStrategy,
  BearerAuth,
  ClaimTokenOutput,
  ConfirmAuthOutput,
  FlowRunTokenAuth,
  PublicKeyAuth,
  PublicKeyProvider,
  RequestOptions,
  StringProvider,
} from "../types.ts";
import { resolvePublicKeyString } from "../internal/runtime.ts";

export function apiKeyAuth(apiKey: StringProvider): ApiKeyAuth {
  return { kind: "apiKey", apiKey };
}

export function bearerAuth(token: StringProvider): BearerAuth {
  return { kind: "bearer", token };
}

export function flowRunTokenAuth(token: StringProvider): FlowRunTokenAuth {
  return { kind: "flowRunToken", token };
}

export function publicKeyAuth(publicKey: PublicKeyProvider): PublicKeyAuth {
  return { kind: "publicKey", publicKey };
}

function resolveSignature(
  signature: string | Uint8Array | ArrayBuffer,
): string {
  if (typeof signature === "string") {
    return signature;
  }
  const bytes = signature instanceof ArrayBuffer
    ? new Uint8Array(signature)
    : signature;
  return encodeBase58(bytes);
}

async function withAnonKeyHeader(
  core: ClientCore,
  headers?: HeadersInit,
): Promise<Headers> {
  const resolved = new Headers(headers);
  if (!resolved.has("apikey")) {
    resolved.set("apikey", await core.resolveAnonKey());
  }
  return resolved;
}

export function createAuthNamespace(core: ClientCore) {
  return {
    async init(
      publicKey: PublicKeyProvider,
      options: Omit<RequestOptions, "auth"> = {},
    ): Promise<string> {
      const result = await core.requestContract(
        z.object({ msg: z.string() }).strict(),
        {
          method: "POST",
          path: "/auth/init",
          auth: false,
          body: { pubkey: await resolvePublicKeyString(publicKey) },
          headers: await withAnonKeyHeader(core, options.headers),
          signal: options.signal,
          retry: options.retry,
          timeoutMs: options.timeoutMs,
        },
        "auth init response",
      );
      return result.msg;
    },

    async confirm(
      message: string,
      signature: string | Uint8Array | ArrayBuffer,
      options: Omit<RequestOptions, "auth"> = {},
    ): Promise<ConfirmAuthOutput> {
      return await core.requestContract(confirmAuthOutputSchema, {
        method: "POST",
        path: "/auth/confirm",
        auth: false,
        body: { token: `${message}.${resolveSignature(signature)}` },
        headers: await withAnonKeyHeader(core, options.headers),
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "auth confirm response") as unknown as ConfirmAuthOutput;
    },

    async loginWithSignature(
      publicKey: PublicKeyProvider,
      signMessage: (
        message: string,
      ) =>
        | string
        | Uint8Array
        | ArrayBuffer
        | Promise<string | Uint8Array | ArrayBuffer>,
      options: Omit<RequestOptions, "auth"> = {},
    ): Promise<ConfirmAuthOutput> {
      const message = await this.init(publicKey, options);
      const signature = await signMessage(message);
      return await this.confirm(message, signature, options);
    },

    async claimToken(options: RequestOptions = {}): Promise<ClaimTokenOutput> {
      return await core.requestContract(claimTokenOutputSchema, {
        method: "POST",
        path: "/auth/claim_token",
        auth: options.auth,
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "claim token response");
    },
  };
}

export type AuthNamespace = ReturnType<typeof createAuthNamespace>;
export type { AuthStrategy };
