import { type IValue, Value } from "../../deps.ts";
import type { FlowInputs, FlowValueInput } from "../../types.ts";

function looksLikeIValue(value: unknown): value is IValue {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const record = value as Record<string, unknown>;
  const keys = Object.keys(record);
  if (keys.length !== 1) {
    return false;
  }

  if (typeof record.S === "string") return true;
  if (typeof record.D === "string") return true;
  if (typeof record.I === "string") return true;
  if (typeof record.U === "string") return true;
  if (typeof record.I1 === "string") return true;
  if (typeof record.U1 === "string") return true;
  if (typeof record.F === "string") return true;
  if (typeof record.B === "boolean") return true;
  if (record.N === 0) return true;
  if (typeof record.B3 === "string") return true;
  if (typeof record.B6 === "string") return true;
  if (typeof record.BY === "string") return true;
  if (Array.isArray(record.A)) return record.A.every(looksLikeIValue);
  if (typeof record.M === "object" && record.M !== null) {
    return Object.values(record.M).every(looksLikeIValue);
  }
  return false;
}

export function normalizeFlowValue(value: FlowValueInput): Value {
  if (value instanceof Value) {
    return value;
  }
  if (looksLikeIValue(value)) {
    return Value.fromJSON(value);
  }
  return new Value(value);
}

export function normalizeFlowInputs(
  inputs?: FlowInputs,
): Record<string, Value> | undefined {
  if (inputs === undefined) {
    return undefined;
  }

  return Object.fromEntries(
    Object.entries(inputs).map((
      [key, value],
    ) => [key, normalizeFlowValue(value)]),
  );
}

export function parseFlowValue(value: IValue): Value {
  return Value.fromJSON(value);
}

export function parseOptionalFlowValue(
  value?: IValue | null,
): Value | undefined {
  if (value === undefined || value === null) {
    return undefined;
  }
  return parseFlowValue(value);
}
