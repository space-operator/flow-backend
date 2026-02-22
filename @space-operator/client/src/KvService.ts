import { Context, Effect, Layer } from "effect";
import type { IValue } from "./deps.ts";
import { SpaceHttpClient } from "./HttpApi.ts";
import type { AuthTokenError, HttpApiError } from "./Errors.ts";
import {
  KvDeleteItemOutput,
  KvReadItemOutput,
  KvWriteItemOutput,
} from "./Schema/Rest.ts";

export interface KvServiceShape {
  readonly createStore: (
    store: string,
  ) => Effect.Effect<void, HttpApiError | AuthTokenError>;

  readonly deleteStore: (
    store: string,
  ) => Effect.Effect<void, HttpApiError | AuthTokenError>;

  readonly writeItem: (
    store: string,
    key: string,
    value: IValue,
  ) => Effect.Effect<typeof KvWriteItemOutput.Type, HttpApiError | AuthTokenError>;

  readonly readItem: (
    store: string,
    key: string,
  ) => Effect.Effect<typeof KvReadItemOutput.Type, HttpApiError | AuthTokenError>;

  readonly deleteItem: (
    store: string,
    key: string,
  ) => Effect.Effect<typeof KvDeleteItemOutput.Type, HttpApiError | AuthTokenError>;
}

export class KvService extends Context.Tag("KvService")<
  KvService,
  KvServiceShape
>() {}

export const KvServiceLive: Layer.Layer<
  KvService,
  never,
  SpaceHttpClient
> = Layer.effect(
  KvService,
  Effect.gen(function* () {
    const http = yield* SpaceHttpClient;

    return {
      createStore: (store) =>
        http.postVoid("/kv/create_store", { store }),

      deleteStore: (store) =>
        http.postVoid("/kv/delete_store", { store }),

      writeItem: (store, key, value) =>
        http.post("/kv/write_item", { store, key, value }, KvWriteItemOutput),

      readItem: (store, key) =>
        http.post("/kv/read_item", { store, key }, KvReadItemOutput),

      deleteItem: (store, key) =>
        http.post("/kv/delete_item", { store, key }, KvDeleteItemOutput),
    };
  }),
);
