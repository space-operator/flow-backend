import { FetchHttpClient } from "@effect/platform";
import { Layer } from "effect";
import { SpaceOperatorConfig, SpaceOperatorConfigFromEnv } from "./Config.ts";
import { SpaceHttpClient, SpaceHttpClientLive } from "./HttpApi.ts";
import { AuthService, AuthServiceLive } from "./AuthService.ts";
import { FlowService, FlowServiceLive } from "./FlowService.ts";
import { KvService, KvServiceLive } from "./KvService.ts";
import { ApiKeyService, ApiKeyServiceLive } from "./ApiKeyService.ts";
import { WalletService, WalletServiceLive } from "./WalletService.ts";
import { WsService, WsServiceLive } from "./WsService.ts";

/** All SDK services. */
export type SpaceOperatorServices =
  | AuthService
  | FlowService
  | KvService
  | ApiKeyService
  | WalletService
  | WsService;

/**
 * Composed layer that provides all SDK services.
 * Requires `SpaceOperatorConfig` and the platform's `HttpClient`.
 */
const InternalLive: Layer.Layer<
  AuthService | FlowService | KvService | ApiKeyService | WalletService | WsService,
  never,
  SpaceOperatorConfig
> = Layer.mergeAll(
  AuthServiceLive,
  FlowServiceLive,
  KvServiceLive,
  ApiKeyServiceLive,
  WalletServiceLive,
  WsServiceLive,
).pipe(
  Layer.provideMerge(SpaceHttpClientLive),
  Layer.provide(FetchHttpClient.layer),
);

/**
 * Full SDK layer — provide `SpaceOperatorConfig` and get all services.
 *
 * Usage:
 * ```ts
 * const program = Effect.gen(function* () {
 *   const flow = yield* FlowService;
 *   return yield* flow.startFlow(id, params);
 * }).pipe(Effect.provide(SpaceOperatorLive), Effect.provide(makeConfig({ token: "..." })));
 * ```
 */
export const SpaceOperatorLive = InternalLive;

/**
 * Convenience: everything from environment variables.
 *
 * Reads:
 * - `SPACE_OPERATOR_HOST` (default: https://dev-api.spaceoperator.com)
 * - `SPACE_OPERATOR_TOKEN` (required)
 * - `SPACE_OPERATOR_ANON_KEY` (optional)
 *
 * Usage:
 * ```ts
 * const program = Effect.gen(function* () {
 *   const flow = yield* FlowService;
 *   return yield* flow.startFlow(id, params);
 * }).pipe(Effect.provide(SpaceOperatorFromEnv));
 * ```
 */
export const SpaceOperatorFromEnv: Layer.Layer<
  AuthService | FlowService | KvService | ApiKeyService | WalletService | WsService
> = InternalLive.pipe(
  Layer.provide(SpaceOperatorConfigFromEnv),
);
