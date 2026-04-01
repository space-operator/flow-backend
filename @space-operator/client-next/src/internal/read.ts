import { iValueSchema } from "@space-operator/contracts";
import { type IValue, Value } from "../deps.ts";
import type {
  AuthStrategy,
  ReadResult,
  RequestOptions,
} from "../types.ts";
import type { ClientCore } from "./core.ts";
import { parseContract } from "./contracts.ts";
import type { JsonRequestOptions } from "./transport/http.ts";
import { resolveAuthHeaders } from "./runtime.ts";

interface CachedReadEntry {
  value: Value;
  etag?: string;
  cacheControl?: string;
  lastModified?: string;
  freshUntil?: number;
}

const readCaches = new WeakMap<ClientCore, Map<string, CachedReadEntry>>();

function getCache(core: ClientCore): Map<string, CachedReadEntry> {
  let cache = readCaches.get(core);
  if (!cache) {
    cache = new Map();
    readCaches.set(core, cache);
  }
  return cache;
}

function parseMaxAge(cacheControl?: string): number | undefined {
  if (!cacheControl) {
    return undefined;
  }
  for (const part of cacheControl.split(",")) {
    const [key, value] = part.trim().split("=", 2);
    if (key.toLowerCase() === "max-age" && value) {
      const parsed = Number.parseInt(value, 10);
      if (Number.isFinite(parsed) && parsed >= 0) {
        return parsed;
      }
    }
  }
  return undefined;
}

function freshUntil(cacheControl?: string): number | undefined {
  const maxAge = parseMaxAge(cacheControl);
  if (maxAge === undefined) {
    return undefined;
  }
  return Date.now() + (maxAge * 1_000);
}

function normalizeHeaders(headers?: HeadersInit): Headers | undefined {
  if (headers === undefined) {
    return undefined;
  }
  return new Headers(headers);
}

function effectiveAuth(
  core: ClientCore,
  options: RequestOptions,
): AuthStrategy | undefined {
  return Object.prototype.hasOwnProperty.call(options, "auth")
    ? options.auth
    : core.config.auth;
}

export async function resolveReadAuthScope(
  core: ClientCore,
  options: RequestOptions,
): Promise<string> {
  const headers = await resolveAuthHeaders(effectiveAuth(core, options));
  const entries = [...headers.entries()]
    .map(([key, value]) => `${key}:${value}`)
    .sort();
  return entries.length > 0 ? entries.join("|") : "none";
}

export async function performReadRequest(
  core: ClientCore,
  {
    cacheKey,
    options,
    request,
    skipCache,
    subject,
  }: {
    cacheKey: string;
    options: RequestOptions;
    request: JsonRequestOptions;
    skipCache: boolean;
    subject: string;
  },
): Promise<ReadResult> {
  const cache = getCache(core);
  const cached = cache.get(cacheKey);
  if (!skipCache && cached?.freshUntil !== undefined && cached.freshUntil > Date.now()) {
    return {
      value: cached.value,
      cached: true,
      etag: cached.etag,
      cacheControl: cached.cacheControl,
      lastModified: cached.lastModified,
    };
  }

  const headers = normalizeHeaders(request.headers) ?? new Headers();
  if (!skipCache && request.method.toUpperCase() === "GET" && cached?.etag) {
    headers.set("If-None-Match", cached.etag);
  }

  const response = await core.requestJsonWithMeta({
    ...request,
    headers,
    auth: effectiveAuth(core, options),
  });

  if (response.status === 304) {
    if (!cached) {
      throw new Error(`${subject} returned 304 without a cached value`);
    }
    const refreshed: CachedReadEntry = {
      value: cached.value,
      etag: response.etag ?? cached.etag,
      cacheControl: response.cacheControl ?? cached.cacheControl,
      lastModified: response.lastModified ?? cached.lastModified,
      freshUntil: freshUntil(response.cacheControl ?? cached.cacheControl),
    };
    cache.set(cacheKey, refreshed);
    return {
      value: refreshed.value,
      cached: true,
      etag: refreshed.etag,
      cacheControl: refreshed.cacheControl,
      lastModified: refreshed.lastModified,
    };
  }

  const parsed = parseContract(
    iValueSchema,
    response.body,
    subject,
  );
  const result: CachedReadEntry = {
    value: Value.fromJSON(parsed as IValue),
    etag: response.etag,
    cacheControl: response.cacheControl,
    lastModified: response.lastModified,
    freshUntil: skipCache ? undefined : freshUntil(response.cacheControl),
  };
  if (!skipCache) {
    cache.set(cacheKey, result);
  }
  return {
    value: result.value,
    cached: false,
    etag: result.etag,
    cacheControl: result.cacheControl,
    lastModified: result.lastModified,
  };
}
