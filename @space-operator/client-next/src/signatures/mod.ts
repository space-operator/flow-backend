import { encodeBase58, encodeBase64 } from "../deps.ts";
import { successResponseSchema } from "@space-operator/contracts";
import type { ClientCore } from "../internal/core.ts";
import type {
  RequestOptions,
  SubmitSignatureInput,
  SuccessResponse,
} from "../types.ts";

function normalizeSignature(value: SubmitSignatureInput["signature"]): string {
  if (typeof value === "string") {
    return value;
  }
  const bytes = value instanceof ArrayBuffer ? new Uint8Array(value) : value;
  return encodeBase58(bytes);
}

function normalizeNewMessage(
  value: SubmitSignatureInput["new_msg"],
): string | undefined {
  if (value === undefined) {
    return undefined;
  }
  if (typeof value === "string") {
    return value;
  }
  const bytes = value instanceof ArrayBuffer ? new Uint8Array(value) : value;
  return encodeBase64(bytes);
}

export function createSignaturesNamespace(core: ClientCore) {
  return {
    async submit(
      input: SubmitSignatureInput,
      options: Omit<RequestOptions, "auth"> = {},
    ): Promise<SuccessResponse> {
      return await core.requestContract(successResponseSchema, {
        method: "POST",
        path: "/signature/submit",
        auth: false,
        body: {
          id: input.id,
          signature: normalizeSignature(input.signature),
          ...(input.new_msg !== undefined
            ? { new_msg: normalizeNewMessage(input.new_msg) }
            : {}),
        },
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "submit signature response");
    },
  };
}

export type SignaturesNamespace = ReturnType<typeof createSignaturesNamespace>;
