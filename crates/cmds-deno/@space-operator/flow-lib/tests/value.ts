import { Value, isIValue } from "../src/value.ts";
import {
  assertEquals,
  assertStrictEquals,
  assert,
  assertThrows,
} from "jsr:@std/assert";

Deno.test("isIValue", () => {
  assert(isIValue({ S: "" }));
  assert(isIValue({ D: "" }));
  assert(isIValue({ I: "" }));
  assert(isIValue({ U: "" }));
  assert(isIValue({ I1: "" }));
  assert(isIValue({ U1: "" }));
  assert(isIValue({ F: "" }));
  assert(isIValue({ B: false }));
  assert(isIValue({ N: 0 }));
  assert(isIValue({ B3: "" }));
  assert(isIValue({ B6: "" }));
  assert(isIValue({ BY: "" }));
  assert(isIValue({ A: [{ S: "" }] }));
  assert(isIValue({ M: { key: { N: 0 } } }));
  assert(!isIValue({}));
  assert(!isIValue({ X: 100 }));
  assert(!isIValue({ S: "", B: false }));
  assert(!isIValue(""));
  assert(!isIValue(false));
  assert(!isIValue(() => 0));
  assert(!isIValue(2));
  assert(!isIValue(2n));
  assert(!isIValue([]));
  assert(!isIValue(null));
  assert(!isIValue(undefined));
  assert(isIValue(Value.Null()));
  assert(isIValue(Value.String("100")));
  assert(isIValue(Value.I64(100)));
  assert(isIValue(Value.U64(100)));
  assert(isIValue(Value.I128(100)));
  assert(isIValue(Value.U128(100)));
  assert(isIValue(Value.Boolean(false)));
  assert(isIValue(Value.Decimal(1.2)));
  assert(isIValue(Value.Float(1.2)));
  assert(isIValue(Value.Bytes(new Uint8Array(32))));
  assert(isIValue(Value.Bytes(new Uint8Array(64))));
  assert(isIValue(Value.Bytes(new Uint8Array(100))));
  assert(isIValue(new Value({ foo: "hello" })));
  assert(isIValue(new Value([Value.I64("10000"), "hello"])));
});

Deno.test("Value.fromJSON", () => {
  const x = new Value();
  assertStrictEquals(Value.fromJSON(x), x);
  assertThrows(() => Value.fromJSON({ S: "string", B: false }));
  assertThrows(() => Value.fromJSON({ B3: "" }));
  assertThrows(() => Value.fromJSON({ F: "" }));
  assertThrows(() => Value.fromJSON({ I: "1.1" }));
  assertThrows(() => Value.fromJSON({ I1: "1.1" }));
});
