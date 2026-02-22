import { Context, Effect, Layer } from "effect";
import { SpaceHttpClient } from "./HttpApi.ts";
import type { AuthTokenError, HttpApiError } from "./Errors.ts";
import {
  ApiKeyInfoOutput,
  CreateApiKeyOutput,
  ServerInfo,
} from "./Schema/Rest.ts";

export interface ApiKeyServiceShape {
  /** Create a new API key with the given name. */
  readonly create: (
    name: string,
  ) => Effect.Effect<typeof CreateApiKeyOutput.Type, HttpApiError | AuthTokenError>;

  /** Delete an API key by its hash. */
  readonly delete: (
    keyHash: string,
  ) => Effect.Effect<void, HttpApiError | AuthTokenError>;

  /** Get info about the current API key (returns user_id). */
  readonly info: () => Effect.Effect<
    typeof ApiKeyInfoOutput.Type,
    HttpApiError | AuthTokenError
  >;

  /** Get server info (public, no auth required). */
  readonly serverInfo: () => Effect.Effect<typeof ServerInfo.Type, HttpApiError>;
}

export class ApiKeyService extends Context.Tag("ApiKeyService")<
  ApiKeyService,
  ApiKeyServiceShape
>() {}

export const ApiKeyServiceLive: Layer.Layer<
  ApiKeyService,
  never,
  SpaceHttpClient
> = Layer.effect(
  ApiKeyService,
  Effect.gen(function* () {
    const http = yield* SpaceHttpClient;

    return {
      create: (name) =>
        http.post("/apikey/create", { name }, CreateApiKeyOutput),

      delete: (keyHash) =>
        http.postVoid("/apikey/delete", { key_hash: keyHash }),

      info: () => http.get("/apikey/info", ApiKeyInfoOutput),

      serverInfo: () => http.get("/info", ServerInfo, /* auth */ false),
    };
  }),
);
