import { type IValue, Value } from "../../deps.ts";
import type { FlowInputs, FlowValueInput } from "../../types.ts";

const HASH_OFFSET_BASIS_64 = 0xcbf29ce484222325n;
const HASH_PRIME_64 = 0x100000001b3n;
const textEncoder = new TextEncoder();

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

function canonicalizeFlowValue(value: IValue): string {
  if (value.S !== undefined) {
    return `{"S":${JSON.stringify(value.S)}}`;
  }
  if (value.D !== undefined) {
    return `{"D":${JSON.stringify(value.D)}}`;
  }
  if (value.I !== undefined) {
    return `{"I":${JSON.stringify(value.I)}}`;
  }
  if (value.U !== undefined) {
    return `{"U":${JSON.stringify(value.U)}}`;
  }
  if (value.I1 !== undefined) {
    return `{"I1":${JSON.stringify(value.I1)}}`;
  }
  if (value.U1 !== undefined) {
    return `{"U1":${JSON.stringify(value.U1)}}`;
  }
  if (value.F !== undefined) {
    return `{"F":${JSON.stringify(value.F)}}`;
  }
  if (value.B !== undefined) {
    return `{"B":${JSON.stringify(value.B)}}`;
  }
  if (value.N !== undefined) {
    return '{"N":0}';
  }
  if (value.B3 !== undefined) {
    return `{"B3":${JSON.stringify(value.B3)}}`;
  }
  if (value.B6 !== undefined) {
    return `{"B6":${JSON.stringify(value.B6)}}`;
  }
  if (value.BY !== undefined) {
    return `{"BY":${JSON.stringify(value.BY)}}`;
  }
  if (value.A !== undefined) {
    return `{"A":[${value.A.map(canonicalizeFlowValue).join(",")}]}`;
  }
  if (value.M !== undefined) {
    const entries = Object.entries(value.M)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, item]) =>
        `${JSON.stringify(key)}:${canonicalizeFlowValue(item)}`
      );
    return `{"M":{${entries.join(",")}}}`;
  }
  throw new TypeError("invalid flow value for stable hashing");
}

function fnv1a64(input: string): string {
  let hash = HASH_OFFSET_BASIS_64;
  for (const byte of textEncoder.encode(input)) {
    hash ^= BigInt(byte);
    hash = BigInt.asUintN(64, hash * HASH_PRIME_64);
  }
  return hash.toString(16).padStart(16, "0");
}

export function stableHash(inputs?: Record<string, Value>): string {
  if (inputs === undefined) {
    return "empty";
  }

  const entries = Object.entries(inputs)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([key, value]) =>
      `${JSON.stringify(key)}:${canonicalizeFlowValue(value)}`
    );

  if (entries.length === 0) {
    return "empty";
  }

  return fnv1a64(`{${entries.join(",")}}`);
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
