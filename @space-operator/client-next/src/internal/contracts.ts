import type { ZodType } from "zod";
import { ContractValidationError } from "./transport/errors.ts";

export function parseContract<T>(
  schema: ZodType<T>,
  value: unknown,
  subject: string,
): T {
  const result = schema.safeParse(value);
  if (!result.success) {
    throw new ContractValidationError(subject, result.error.issues);
  }
  return result.data;
}
