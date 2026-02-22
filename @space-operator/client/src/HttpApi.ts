import {
  HttpClient,
  HttpClientRequest,
  HttpClientResponse,
} from "@effect/platform";
import { Context, Effect, Layer, Redacted, Schema } from "effect";
import { SpaceOperatorConfig } from "./Config.ts";
import { AuthTokenError, HttpApiError } from "./Errors.ts";

// --- Header Logic ---

/** API keys prefixed with "b3-" use the x-api-key header; everything else is Bearer. */
function authHeader(
  token: string,
): { name: string; value: string } {
  if (token.startsWith("b3-")) {
    return { name: "x-api-key", value: token };
  }
  return { name: "authorization", value: token };
}

// --- Service Definition ---

export interface SpaceHttpClientShape {
  /** Authenticated GET, decoded with schema. */
  readonly get: <A, I, R>(
    path: string,
    schema: Schema.Schema<A, I, R>,
    opts?: boolean | { auth?: boolean; customToken?: string },
  ) => Effect.Effect<A, HttpApiError | AuthTokenError>;

  /** Authenticated POST with JSON body, decoded with schema. */
  readonly post: <A, I, R>(
    path: string,
    body: unknown,
    schema: Schema.Schema<A, I, R>,
    opts?: { auth?: boolean; anonKey?: boolean; customToken?: string },
  ) => Effect.Effect<A, HttpApiError | AuthTokenError>;

  /** Authenticated POST with JSON body, no response parsing (returns void). */
  readonly postVoid: (
    path: string,
    body?: unknown,
    opts?: { auth?: boolean; anonKey?: boolean; customToken?: string },
  ) => Effect.Effect<void, HttpApiError | AuthTokenError>;
}

export class SpaceHttpClient extends Context.Tag("SpaceHttpClient")<
  SpaceHttpClient,
  SpaceHttpClientShape
>() {}

// --- Implementation ---

export const SpaceHttpClientLive: Layer.Layer<
  SpaceHttpClient,
  never,
  SpaceOperatorConfig | HttpClient.HttpClient
> = Layer.effect(
  SpaceHttpClient,
  Effect.gen(function* () {
    const config = yield* SpaceOperatorConfig;
    const client = yield* HttpClient.HttpClient;

    /** Resolve auth token from config. */
    const getToken = (): Effect.Effect<string, AuthTokenError> =>
      Effect.try({
        try: () => Redacted.value(config.token),
        catch: () => new AuthTokenError({ message: "no authentication token" }),
      });

    const getAnonKey = (): Effect.Effect<string, AuthTokenError> =>
      config.anonKey
        ? Effect.succeed(Redacted.value(config.anonKey))
        : Effect.fail(
            new AuthTokenError({ message: "no anon key configured" }),
          );

    /** Apply auth header to a request. */
    const withAuth = (
      req: HttpClientRequest.HttpClientRequest,
      auth: boolean,
    ): Effect.Effect<
      HttpClientRequest.HttpClientRequest,
      AuthTokenError
    > => {
      if (!auth) return Effect.succeed(req);
      return Effect.map(getToken(), (token) => {
        const { name, value } = authHeader(token);
        return HttpClientRequest.setHeader(name, value)(req);
      });
    };

    const withAnonKey = (
      req: HttpClientRequest.HttpClientRequest,
      anonKey: boolean,
    ): Effect.Effect<
      HttpClientRequest.HttpClientRequest,
      AuthTokenError
    > => {
      if (!anonKey) return Effect.succeed(req);
      return Effect.map(getAnonKey(), (key) =>
        HttpClientRequest.setHeader("apikey", key)(req),
      );
    };

    /** Parse error response into HttpApiError. */
    const handleError = (
      response: HttpClientResponse.HttpClientResponse,
      url: string,
    ): Effect.Effect<never, HttpApiError> =>
      Effect.flatMap(
        response.text.pipe(Effect.orElseSucceed(() => "")),
        (bodyText) => {
          let message: string;
          const contentType = response.headers["content-type"] ?? "";
          if (contentType.includes("application/json")) {
            try {
              const body = JSON.parse(bodyText);
              message = body?.error ?? bodyText;
            } catch {
              message = bodyText;
            }
          } else {
            message = bodyText || `HTTP ${response.status}`;
          }
          return Effect.fail(
            new HttpApiError({
              status: response.status,
              url,
              body: bodyText,
              message,
            }),
          );
        },
      );

    /** Execute request and handle response. */
    const execute = <A, I, R>(
      req: HttpClientRequest.HttpClientRequest,
      schema: Schema.Schema<A, I, R>,
    ): Effect.Effect<A, HttpApiError> =>
      Effect.gen(function* () {
        const response = yield* client.execute(req);
        if (response.status >= 200 && response.status < 300) {
          return yield* HttpClientResponse.schemaBodyJson(schema)(response).pipe(
            Effect.mapError((e) =>
              new HttpApiError({
                status: response.status,
                url: req.url,
                body: String(e),
                message: `Schema decode error: ${e}`,
              })
            ),
          );
        }
        return yield* handleError(response, req.url);
      }).pipe(
        Effect.catchTag("RequestError", (e) =>
          Effect.fail(
            new HttpApiError({
              status: 0,
              url: req.url,
              body: "",
              message: `Request failed: ${e.message}`,
            }),
          )
        ),
        Effect.catchTag("ResponseError", (e) =>
          Effect.fail(
            new HttpApiError({
              status: 0,
              url: req.url,
              body: "",
              message: `Response error: ${e.message}`,
            }),
          )
        ),
      );

    const executeVoid = (
      req: HttpClientRequest.HttpClientRequest,
    ): Effect.Effect<void, HttpApiError> =>
      Effect.gen(function* () {
        const response = yield* client.execute(req);
        if (response.status >= 200 && response.status < 300) {
          return;
        }
        return yield* handleError(response, req.url);
      }).pipe(
        Effect.catchTag("RequestError", (e) =>
          Effect.fail(
            new HttpApiError({
              status: 0,
              url: req.url,
              body: "",
              message: `Request failed: ${e.message}`,
            }),
          )
        ),
        Effect.catchTag("ResponseError", (e) =>
          Effect.fail(
            new HttpApiError({
              status: 0,
              url: req.url,
              body: "",
              message: `Response error: ${e.message}`,
            }),
          )
        ),
      );

    return {
      get: (path, schema, opts) =>
        Effect.gen(function* () {
          const url = `${config.host}${path}`;
          let req = HttpClientRequest.get(url);
          // opts can be a boolean (legacy: auth flag) or an options object
          const resolved = typeof opts === "boolean"
            ? { auth: opts }
            : (opts ?? {});
          if (resolved.customToken) {
            const { name, value } = authHeader(resolved.customToken);
            req = HttpClientRequest.setHeader(name, value)(req);
          } else {
            req = yield* withAuth(req, resolved.auth ?? true);
          }
          return yield* execute(req, schema);
        }),

      post: (path, body, schema, opts) =>
        Effect.gen(function* () {
          const url = `${config.host}${path}`;
          let req = HttpClientRequest.post(url);
          if (body !== undefined) {
            req = yield* HttpClientRequest.bodyJson(req, body).pipe(
              Effect.mapError(() =>
                new AuthTokenError({ message: "failed to serialize body" })
              ),
            );
          }
          if (opts?.customToken) {
            const { name, value } = authHeader(opts.customToken);
            req = HttpClientRequest.setHeader(name, value)(req);
          } else {
            req = yield* withAuth(req, opts?.auth ?? true);
          }
          req = yield* withAnonKey(req, opts?.anonKey ?? false);
          return yield* execute(req, schema);
        }),

      postVoid: (path, body, opts) =>
        Effect.gen(function* () {
          const url = `${config.host}${path}`;
          let req = HttpClientRequest.post(url);
          if (body !== undefined) {
            req = yield* HttpClientRequest.bodyJson(req, body).pipe(
              Effect.mapError(() =>
                new AuthTokenError({ message: "failed to serialize body" })
              ),
            );
          }
          if (opts?.customToken) {
            const { name, value } = authHeader(opts.customToken);
            req = HttpClientRequest.setHeader(name, value)(req);
          } else {
            req = yield* withAuth(req, opts?.auth ?? true);
          }
          req = yield* withAnonKey(req, opts?.anonKey ?? false);
          return yield* executeVoid(req);
        }),
    };
  }),
);
