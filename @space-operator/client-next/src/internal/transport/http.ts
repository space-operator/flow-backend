import type { AuthStrategy, RequestOptions, RetryPolicy } from "../../types.ts";
import { Effect, Either, runClientEffect } from "../effect.ts";
import {
  log,
  resolveAuthHeaders,
  type ResolvedClientConfig,
  sleep,
} from "../runtime.ts";
import { withSpan } from "../telemetry.ts";
import {
  AbortError,
  ApiError,
  ClientError,
  TimeoutError,
  TransportError,
} from "./errors.ts";

export interface JsonRequestOptions extends Omit<RequestOptions, "auth"> {
  auth?: AuthStrategy | false;
  body?: unknown;
  method: string;
  path: string;
  query?:
    | URLSearchParams
    | Record<string, string | number | boolean | null | undefined>;
}

function mergeRetryPolicy(
  base: RetryPolicy | undefined,
  override: RetryPolicy | undefined,
): RetryPolicy | undefined {
  if (base === undefined) return override;
  if (override === undefined) return base;
  return {
    attempts: override.attempts ?? base.attempts,
    backoffMs: override.backoffMs ?? base.backoffMs,
    retryableStatusCodes: override.retryableStatusCodes ??
      base.retryableStatusCodes,
  };
}

function buildUrl(
  config: ResolvedClientConfig,
  path: string,
  query?: JsonRequestOptions["query"],
): URL {
  const url = new URL(path.replace(/^\/+/, ""), `${config.baseUrl}/`);
  if (query instanceof URLSearchParams) {
    url.search = query.toString();
    return url;
  }

  if (query !== undefined) {
    for (const [key, value] of Object.entries(query)) {
      if (value !== undefined && value !== null) {
        url.searchParams.set(key, String(value));
      }
    }
  }
  return url;
}

function parseBackoff(
  backoff: RetryPolicy["backoffMs"],
  attempt: number,
): number {
  if (typeof backoff === "function") {
    return backoff(attempt);
  }
  if (typeof backoff === "number") {
    return backoff;
  }
  return 0;
}

function isJsonResponse(contentType: string | null): boolean {
  return contentType?.toLowerCase().includes("application/json") === true;
}

async function parseResponseBody(response: Response): Promise<unknown> {
  const text = await response.text();
  if (text.length === 0) {
    return undefined;
  }
  if (isJsonResponse(response.headers.get("content-type"))) {
    try {
      return JSON.parse(text);
    } catch {
      return text;
    }
  }
  return text;
}

function extractErrorMessage(body: unknown, fallback: string): string {
  if (typeof body === "string" && body.length > 0) {
    return body;
  }
  if (
    typeof body === "object" &&
    body !== null &&
    "error" in body &&
    typeof (body as { error?: unknown }).error === "string"
  ) {
    return (body as { error: string }).error;
  }
  return fallback;
}

function normalizeUnknownError(error: unknown): Error {
  if (error instanceof Error) {
    return error;
  }
  return new Error(String(error));
}

function normalizeClientError(
  error: unknown,
  fallback: string,
): ClientError {
  if (error instanceof ClientError) {
    return error;
  }
  return new TransportError(fallback, { cause: normalizeUnknownError(error) });
}

function delayEffect(ms: number) {
  if (ms <= 0) {
    return Effect.succeed(undefined);
  }

  return Effect.tryPromise({
    try: () => sleep(ms),
    catch: (error) =>
      normalizeClientError(error, `failed to wait ${ms}ms before retry`),
  });
}

function resolveAuthHeadersEffect(auth?: AuthStrategy) {
  return Effect.tryPromise({
    try: () => resolveAuthHeaders(auth),
    catch: (error) =>
      normalizeClientError(error, "failed to resolve auth headers"),
  });
}

function parseResponseBodyEffect(
  response: Response,
  method: string,
  url: URL,
) {
  return Effect.tryPromise({
    try: () => parseResponseBody(response),
    catch: (error) =>
      new TransportError(
        `${method} ${url} failed while reading the response body`,
        {
          cause: normalizeUnknownError(error),
        },
      ),
  });
}

function fetchEffect(
  config: ResolvedClientConfig,
  {
    body,
    headers,
    method,
    options,
    url,
  }: {
    body: BodyInit | undefined;
    headers: Headers;
    method: string;
    options: JsonRequestOptions;
    url: URL;
  },
) {
  return Effect.tryPromise({
    try: async () => {
      const controller = new AbortController();
      if (options.signal) {
        if (options.signal.aborted) {
          controller.abort(options.signal.reason);
        } else {
          options.signal.addEventListener(
            "abort",
            () => controller.abort(options.signal?.reason),
            { once: true },
          );
        }
      }

      const signal = controller.signal;
      let timeoutId: number | undefined;
      if (options.timeoutMs ?? config.timeoutMs) {
        timeoutId = setTimeout(
          () => controller.abort("timeout"),
          options.timeoutMs ?? config.timeoutMs,
        ) as unknown as number;
      }

      log(config, {
        scope: "http",
        event: "request",
        data: { method, url: url.toString() },
      });

      try {
        return await config.fetch(url, {
          method,
          body,
          headers,
          signal,
        });
      } catch (error) {
        if (error instanceof DOMException && error.name === "AbortError") {
          if (options.signal?.aborted) {
            throw new AbortError(`${method} ${url} was aborted`, {
              cause: error,
            });
          }
          if (controller.signal.aborted) {
            throw new TimeoutError(`${method} ${url} timed out`);
          }
          throw new AbortError(`${method} ${url} was aborted`, {
            cause: error,
          });
        }

        throw new TransportError(`${method} ${url} failed`, {
          cause: normalizeUnknownError(error),
        });
      } finally {
        if (timeoutId !== undefined) {
          clearTimeout(timeoutId);
        }
      }
    },
    catch: (error) => normalizeClientError(error, `${method} ${url} failed`),
  });
}

function describeAuthKind(auth?: AuthStrategy | false): string {
  if (auth === false || auth === undefined) {
    return "none";
  }
  return auth.kind;
}

export interface JsonResponseWithMeta<T> {
  body: T | undefined;
  cacheControl?: string;
  etag?: string;
  headers: Headers;
  lastModified?: string;
  status: number;
}

async function requestJsonInternal<T>(
  config: ResolvedClientConfig,
  options: JsonRequestOptions,
  allowNotModified: boolean,
): Promise<JsonResponseWithMeta<T>> {
  const retry = mergeRetryPolicy(config.retry, options.retry);
  const attempts = Math.max(retry?.attempts ?? 1, 1);
  const method = options.method.toUpperCase();
  const url = buildUrl(config, options.path, options.query);

  const fetchResponseAttempt = (
    attempt: number,
    onStatus?: (status: number) => void,
  ): Effect.Effect<Response, ClientError> =>
    Effect.gen(function* () {
      const headers = new Headers(options.headers);
      const effectiveAuth = options.auth === false
        ? undefined
        : options.auth ?? config.auth;
      const authHeaders = yield* resolveAuthHeadersEffect(effectiveAuth);
      authHeaders.forEach((value, key) => headers.set(key, value));

      let body: BodyInit | undefined;
      if (options.body !== undefined) {
        headers.set("content-type", "application/json");
        body = JSON.stringify(options.body);
      }

      log(config, {
        scope: "http",
        event: "attempt",
        data: { attempt, method, url: url.toString() },
      });

      const result = yield* Effect.either(
        fetchEffect(config, {
          body,
          headers,
          method,
          options,
          url,
        }),
      );
      if (Either.isLeft(result)) {
        const error = normalizeClientError(
          result.left,
          `${method} ${url} failed`,
        );
        if (attempt < attempts && error instanceof TransportError) {
          yield* delayEffect(parseBackoff(retry?.backoffMs, attempt));
          return yield* fetchResponseAttempt(attempt + 1, onStatus);
        }
        return yield* Effect.fail(error);
      }
      const response = result.right;
      onStatus?.(response.status);
      return response;
    }) as Effect.Effect<Response, ClientError>;

  const requestAttempt = (
    attempt: number,
    onStatus?: (status: number) => void,
  ): Effect.Effect<JsonResponseWithMeta<T>, ClientError> =>
    Effect.gen(function* () {
      const response = yield* fetchResponseAttempt(attempt, onStatus);
      if (allowNotModified && response.status === 304) {
        return {
          body: undefined,
          cacheControl: response.headers.get("cache-control") ?? undefined,
          etag: response.headers.get("etag") ?? undefined,
          headers: response.headers,
          lastModified: response.headers.get("last-modified") ?? undefined,
          status: response.status,
        };
      }

      if (response.ok) {
        const parsed = yield* parseResponseBodyEffect(response, method, url);
        return {
          body: parsed as T,
          cacheControl: response.headers.get("cache-control") ?? undefined,
          etag: response.headers.get("etag") ?? undefined,
          headers: response.headers,
          lastModified: response.headers.get("last-modified") ?? undefined,
          status: response.status,
        };
      }

      const parsedBody = yield* parseResponseBodyEffect(response, method, url);
      const apiError = new ApiError(
        extractErrorMessage(
          parsedBody,
          `${method} ${url} failed with ${response.status}`,
        ),
        {
          status: response.status,
          statusText: response.statusText,
          url: url.toString(),
          method,
          requestId: response.headers.get("x-request-id") ?? undefined,
          body: parsedBody,
        },
      );

      if (
        attempt < attempts &&
        response.status !== 304 &&
        (retry?.retryableStatusCodes ?? [429, 500, 502, 503, 504]).includes(
          response.status,
        )
      ) {
        yield* delayEffect(parseBackoff(retry?.backoffMs, attempt));
        return yield* requestAttempt(attempt + 1, onStatus);
      }

      return yield* Effect.fail(apiError);
    });

  return await withSpan(
    config.telemetry,
    "space_operator.http.request",
    {
      "http.request.method": method,
      "url.full": url.toString(),
      "space_operator.http.path": options.path,
      "space_operator.auth.kind": describeAuthKind(options.auth ?? config.auth),
      "space_operator.retry.attempts": attempts,
    },
    async (span) => {
      return await runClientEffect(
        requestAttempt(
          1,
          (status) => span.setAttribute("http.response.status_code", status),
        ),
      );
    },
  );
}

export async function requestJson<T>(
  config: ResolvedClientConfig,
  options: JsonRequestOptions,
): Promise<T> {
  const response = await requestJsonInternal<T>(config, options, false);
  return response.body as T;
}

export async function requestJsonWithMeta<T>(
  config: ResolvedClientConfig,
  options: JsonRequestOptions,
): Promise<JsonResponseWithMeta<T>> {
  return await requestJsonInternal<T>(config, options, true);
}
