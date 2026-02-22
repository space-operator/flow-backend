import { Context, Effect, Layer } from "effect";
import { SpaceHttpClient } from "./HttpApi.ts";
import type { AuthTokenError, HttpApiError } from "./Errors.ts";
import {
  type UpsertWalletBody,
  UpsertWalletResponse,
} from "./Schema/Wallet.ts";

export interface WalletServiceShape {
  /** Create or update a wallet entry. */
  readonly upsertWallet: (
    body: UpsertWalletBody,
  ) => Effect.Effect<typeof UpsertWalletResponse.Type, HttpApiError | AuthTokenError>;
}

export class WalletService extends Context.Tag("WalletService")<
  WalletService,
  WalletServiceShape
>() {}

export const WalletServiceLive: Layer.Layer<
  WalletService,
  never,
  SpaceHttpClient
> = Layer.effect(
  WalletService,
  Effect.gen(function* () {
    const http = yield* SpaceHttpClient;

    return {
      upsertWallet: (body) =>
        http.post("/wallets/upsert", body, UpsertWalletResponse),
    };
  }),
);
