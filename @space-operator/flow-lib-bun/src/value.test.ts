import { test, expect, describe } from "bun:test";
import { Value, isIValue } from "./value.ts";

describe("Value constructors", () => {
  test("String", () => {
    const v = Value.String("hello");
    expect(v.S).toBe("hello");
    expect(v.asString()).toBe("hello");
  });

  test("Boolean", () => {
    expect(Value.Boolean(true).B).toBe(true);
    expect(Value.Boolean(false).B).toBe(false);
    expect(Value.Boolean(true).asBool()).toBe(true);
  });

  test("Null", () => {
    const v = Value.Null();
    expect(v.N).toBe(0);
  });

  test("U64", () => {
    const v = Value.U64(42);
    expect(v.U).toBe("42");
    expect(v.asNumber()).toBe(42);
    expect(v.asBigInt()).toBe(42n);
  });

  test("U64 from bigint", () => {
    const v = Value.U64(1_000_000n);
    expect(v.U).toBe("1000000");
  });

  test("U64 rejects negative", () => {
    expect(() => Value.U64(-1)).toThrow();
  });

  test("I64", () => {
    const v = Value.I64(-99);
    expect(v.I).toBe("-99");
    expect(v.asNumber()).toBe(-99);
    expect(v.asBigInt()).toBe(-99n);
  });

  test("U128", () => {
    const big = 340282366920938463463374607431768211455n;
    const v = Value.U128(big);
    expect(v.U1).toBe(big.toString());
    expect(v.asBigInt()).toBe(big);
  });

  test("I128", () => {
    const v = Value.I128(-1000n);
    expect(v.I1).toBe("-1000");
  });

  test("Float", () => {
    const v = Value.Float(3.14);
    expect(v.F).toBe("3.14");
    expect(v.asNumber()).toBeCloseTo(3.14);
  });

  test("Decimal", () => {
    const v = Value.Decimal(2.718);
    expect(v.D).toBe("2.718");
  });
});

describe("Value inference (constructor)", () => {
  test("number → Decimal", () => {
    const v = new Value(42);
    expect(v.D).toBe("42");
  });

  test("boolean → Boolean", () => {
    const v = new Value(true);
    expect(v.B).toBe(true);
  });

  test("string → String", () => {
    const v = new Value("abc");
    expect(v.S).toBe("abc");
  });

  test("null → Null", () => {
    const v = new Value(null);
    expect(v.N).toBe(0);
  });

  test("undefined → Null", () => {
    const v = new Value(undefined);
    expect(v.N).toBe(0);
  });

  test("positive bigint → U128", () => {
    const v = new Value(100n);
    expect(v.U1).toBe("100");
  });

  test("negative bigint → I128", () => {
    const v = new Value(-50n);
    expect(v.I1).toBe("-50");
  });

  test("array → A", () => {
    const v = new Value([1, "two", true]);
    expect(v.A).toBeArrayOfSize(3);
    expect(v.A![0].D).toBe("1");
    expect(v.A![1].S).toBe("two");
    expect(v.A![2].B).toBe(true);
  });

  test("object → M", () => {
    const v = new Value({ x: 1, y: "z" });
    expect(v.M).toBeDefined();
    expect(v.M!.x.D).toBe("1");
    expect(v.M!.y.S).toBe("z");
  });
});

describe("Value.fromJSON", () => {
  test("round-trips String", () => {
    const v = Value.fromJSON({ S: "test" });
    expect(v).toBeInstanceOf(Value);
    expect(v.asString()).toBe("test");
  });

  test("round-trips nested Map", () => {
    const v = Value.fromJSON({
      M: {
        name: { S: "alice" },
        age: { U: "30" },
      },
    });
    expect(v.M).toBeDefined();
    expect(v.M!.name.asString()).toBe("alice");
    expect(v.M!.age.asNumber()).toBe(30);
  });

  test("round-trips Array", () => {
    const v = Value.fromJSON({ A: [{ S: "a" }, { U: "1" }] });
    expect(v.A).toBeArrayOfSize(2);
    expect(v.A![0].asString()).toBe("a");
    expect(v.A![1].asBigInt()).toBe(1n);
  });

  test("rejects invalid IValue", () => {
    expect(() => Value.fromJSON({ X: "bad" } as any)).toThrow();
  });
});

describe("isIValue", () => {
  test("valid String", () => expect(isIValue({ S: "x" })).toBe(true));
  test("valid Bool", () => expect(isIValue({ B: false })).toBe(true));
  test("valid Null", () => expect(isIValue({ N: 0 })).toBe(true));
  test("valid U64", () => expect(isIValue({ U: "5" })).toBe(true));
  test("rejects empty obj", () => expect(isIValue({})).toBe(false));
  test("rejects two keys", () => expect(isIValue({ S: "x", U: "1" })).toBe(false));
  test("rejects primitive", () => expect(isIValue("str")).toBe(false));
  test("rejects null", () => expect(isIValue(null)).toBe(false));
});

describe("toJSObject", () => {
  test("String → string", () => {
    expect(Value.String("hello").toJSObject()).toBe("hello");
  });

  test("U64 → number", () => {
    expect(Value.U64(42).toJSObject()).toBe(42);
  });

  test("Boolean → boolean", () => {
    expect(Value.Boolean(true).toJSObject()).toBe(true);
  });

  test("Null → null", () => {
    expect(Value.Null().toJSObject()).toBeNull();
  });

  test("nested map → object", () => {
    const v = Value.fromJSON({ M: { k: { S: "v" } } });
    const obj = v.toJSObject();
    expect(obj).toEqual({ k: "v" });
  });

  test("array → array", () => {
    const v = Value.fromJSON({ A: [{ S: "a" }, { S: "b" }] });
    expect(v.toJSObject()).toEqual(["a", "b"]);
  });
});
