import { web3 } from "../deps.ts";
import type {
  AuthStrategy,
  ClientLoggerEvent,
  ClientTelemetryOptions,
  CreateClientOptions,
  PublicKeyInput,
  RetryPolicy,
  ValueProvider,
  WebSocketFactory,
  WebSocketLike,
} from "../types.ts";
import {
  type ResolvedTelemetryConfig,
  resolveTelemetryConfig,
} from "./telemetry.ts";

export interface ResolvedClientConfig {
  baseUrl: string;
  auth?: AuthStrategy;
  anonKey?: ValueProvider<string>;
  fetch: typeof globalThis.fetch;
  webSocketFactory?: WebSocketFactory;
  logger?: (entry: ClientLoggerEvent) => void;
  telemetry: ResolvedTelemetryConfig;
  retry?: RetryPolicy;
  timeoutMs?: number;
}

export function normalizeBaseUrl(baseUrl: string): string {
  return baseUrl.replace(/\/+$/, "");
}

export function resolveClientConfig(
  options: CreateClientOptions,
): ResolvedClientConfig {
  const fetchImpl = options.fetch ?? globalThis.fetch;
  if (typeof fetchImpl !== "function") {
    throw new Error("fetch is not available; pass createClient({ fetch })");
  }

  return {
    baseUrl: normalizeBaseUrl(options.baseUrl),
    auth: options.auth,
    anonKey: options.anonKey,
    fetch: fetchImpl,
    webSocketFactory: options.webSocketFactory,
    logger: options.logger,
    telemetry: resolveTelemetryConfig(options.telemetry),
    retry: options.retry,
    timeoutMs: options.timeoutMs,
  };
}

export async function resolveProvider<T>(
  provider: ValueProvider<T>,
): Promise<T> {
  if (typeof provider === "function") {
    return await (provider as () => T | Promise<T>)();
  }
  return provider;
}

export function normalizeBearerToken(token: string): string {
  return token.startsWith("Bearer ") ? token.slice(7) : token;
}

export async function resolvePublicKeyString(
  input: ValueProvider<PublicKeyInput>,
): Promise<string> {
  const value = await resolveProvider(input);
  if (typeof value === "string") {
    return value;
  }
  if (value instanceof web3.PublicKey) {
    return value.toBase58();
  }
  throw new TypeError("expected a base58 string or web3.PublicKey");
}

export async function resolveAuthHeaders(
  auth?: AuthStrategy,
): Promise<Headers> {
  const headers = new Headers();
  if (auth === undefined) {
    return headers;
  }

  switch (auth.kind) {
    case "apiKey": {
      const apiKey = await resolveProvider(auth.apiKey);
      headers.set("x-api-key", apiKey);
      return headers;
    }
    case "bearer": {
      const token = normalizeBearerToken(await resolveProvider(auth.token));
      headers.set("authorization", `Bearer ${token}`);
      return headers;
    }
    case "flowRunToken": {
      const token = normalizeBearerToken(await resolveProvider(auth.token));
      headers.set("authorization", `Bearer ${token}`);
      return headers;
    }
    case "publicKey": {
      const publicKey = await resolvePublicKeyString(auth.publicKey);
      headers.set("authorization", `Bearer ${publicKey}`);
      return headers;
    }
  }
}

export async function resolveWsToken(
  auth?: AuthStrategy,
): Promise<string | undefined> {
  if (auth === undefined) {
    return undefined;
  }

  switch (auth.kind) {
    case "apiKey":
      return await resolveProvider(auth.apiKey);
    case "bearer":
      return normalizeBearerToken(await resolveProvider(auth.token));
    case "flowRunToken":
      return normalizeBearerToken(await resolveProvider(auth.token));
    case "publicKey":
      return undefined;
  }
}

export function log(config: ResolvedClientConfig, entry: ClientLoggerEvent) {
  config.logger?.(entry);
}

export function getWebSocketFactory(
  config: ResolvedClientConfig,
): WebSocketFactory {
  if (config.webSocketFactory) {
    return config.webSocketFactory;
  }
  if (typeof globalThis.WebSocket === "function") {
    return (url: string) =>
      new globalThis.WebSocket(url) as unknown as WebSocketLike;
  }
  throw new Error(
    "WebSocket is not available; pass createClient({ webSocketFactory })",
  );
}

export function toWsUrl(baseUrl: string): string {
  const url = new URL(baseUrl);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  url.pathname = `${url.pathname.replace(/\/$/, "")}/ws`;
  return url.toString();
}

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
