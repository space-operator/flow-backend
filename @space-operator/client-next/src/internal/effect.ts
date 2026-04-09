import { Cause, Effect, Either, Exit, Option } from "effect";

export { Cause, Effect, Either, Exit, Option };

export async function runClientEffect<A, E>(
  effect: Effect.Effect<A, E>,
): Promise<A> {
  const exit = await Effect.runPromiseExit(effect as Effect.Effect<A, E>);
  if (Exit.isSuccess(exit)) {
    return exit.value;
  }

  const failure = Cause.failureOption(exit.cause);
  if (Option.isSome(failure)) {
    throw failure.value;
  }

  const defect = Cause.dieOption(exit.cause);
  if (Option.isSome(defect)) {
    throw defect.value instanceof Error
      ? defect.value
      : new Error(String(defect.value));
  }

  throw new Error("effect interrupted");
}
