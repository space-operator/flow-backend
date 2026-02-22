import { Config, Context, Effect, Layer, Option, Redacted } from "effect";

export interface SpaceOperatorConfigShape {
  readonly host: string;
  /** API key or JWT token (kept redacted in memory). */
  readonly token: Redacted.Redacted<string>;
  /** Supabase anon key — needed for auth endpoints. */
  readonly anonKey?: Redacted.Redacted<string>;
  /** Override the WebSocket URL (defaults to host with ws:// scheme + /ws). */
  readonly wsUrl?: string;
}

export class SpaceOperatorConfig extends Context.Tag("SpaceOperatorConfig")<
  SpaceOperatorConfig,
  SpaceOperatorConfigShape
>() {}

const DEFAULT_HOST = "https://dev-api.spaceoperator.com";

/** Build config from environment variables. */
export const SpaceOperatorConfigFromEnv: Layer.Layer<SpaceOperatorConfig> =
  Layer.effect(
    SpaceOperatorConfig,
    Effect.gen(function* () {
      const host = yield* Config.string("SPACE_OPERATOR_HOST").pipe(
        Config.withDefault(DEFAULT_HOST),
      );
      const token = yield* Config.redacted("SPACE_OPERATOR_TOKEN");
      const anonKey = yield* Config.redacted("SPACE_OPERATOR_ANON_KEY").pipe(
        Config.option,
      );
      return {
        host,
        token,
        anonKey: Option.getOrUndefined(anonKey),
      };
    }),
  );

/** Build config from explicit values (useful for tests and programmatic use). */
export const makeConfig = (opts: {
  host?: string;
  token: string;
  anonKey?: string;
  wsUrl?: string;
}): Layer.Layer<SpaceOperatorConfig> =>
  Layer.succeed(SpaceOperatorConfig, {
    host: opts.host ?? DEFAULT_HOST,
    token: Redacted.make(opts.token),
    anonKey: opts.anonKey ? Redacted.make(opts.anonKey) : undefined,
    wsUrl: opts.wsUrl,
  });
