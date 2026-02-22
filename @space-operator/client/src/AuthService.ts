import { Context, Effect, Layer } from "effect";
import { bs58 } from "./deps.ts";
import { SpaceHttpClient } from "./HttpApi.ts";
import type { AuthTokenError, HttpApiError } from "./Errors.ts";
import {
  ClaimTokenOutput,
  ConfirmAuthOutput,
  InitAuthOutput,
} from "./Schema/Rest.ts";

export interface AuthServiceShape {
  /** Step 1 of Solana auth: get the message to sign. */
  readonly initAuth: (
    pubkey: string,
  ) => Effect.Effect<string, HttpApiError>;

  /** Step 2 of Solana auth: submit message + signature, get Supabase session. */
  readonly confirmAuth: (
    msg: string,
    signature: ArrayBuffer | Uint8Array | string,
  ) => Effect.Effect<typeof ConfirmAuthOutput.Type, HttpApiError>;

  /** Exchange an API key for a short-lived JWT access token. */
  readonly claimToken: () => Effect.Effect<
    typeof ClaimTokenOutput.Type,
    HttpApiError | AuthTokenError
  >;
}

export class AuthService extends Context.Tag("AuthService")<
  AuthService,
  AuthServiceShape
>() { }

export const AuthServiceLive: Layer.Layer<
  AuthService,
  never,
  SpaceHttpClient
> = Layer.effect(
  AuthService,
  Effect.gen(function* () {
    const http = yield* SpaceHttpClient;

    return {
      initAuth: (pubkey) =>
        http
          .post("/auth/init", { pubkey }, InitAuthOutput, {
            auth: false,
            anonKey: true,
          })
          .pipe(Effect.map((r) => r.msg)),

      confirmAuth: (msg, signature) => {
        const sig = typeof signature === "string"
          ? signature
          : bs58.encodeBase58(new Uint8Array(signature));
        const token = `${msg}.${sig}`;
        return http.post("/auth/confirm", { token }, ConfirmAuthOutput, {
          auth: false,
          anonKey: true,
        });
      },

      claimToken: () => http.post("/auth/claim_token", undefined, ClaimTokenOutput),
    };
  }),
);
