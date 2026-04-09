import { test, expect, describe } from "bun:test";
import {
  BaseCommand,
  deserializeInput,
  serializeOutput,
  type NodeData,
} from "./command.ts";
import { Value } from "./value.ts";
import type { Context } from "./context.ts";

function makeNodeData(
  inputs: NodeData["inputs"] = [],
  outputs: NodeData["outputs"] = []
): NodeData {
  return {
    type: "bun",
    node_id: "test-node",
    inputs,
    outputs,
    config: {},
  };
}

describe("BaseCommand", () => {
  test("throws unimplemented by default", () => {
    const cmd = new BaseCommand(makeNodeData());
    expect(() => cmd.run({} as Context, {})).toThrow("unimplemented");
  });

  test("deserializeInputs uses type_bounds", () => {
    const nd = makeNodeData([
      { id: "1", name: "count", type_bounds: ["u64"], required: true, passthrough: false },
      { id: "2", name: "label", type_bounds: ["string"], required: true, passthrough: false },
    ]);
    const cmd = new BaseCommand(nd);
    const inputs = {
      count: Value.U64(42),
      label: Value.String("test"),
    };
    const result = cmd.deserializeInputs(inputs);
    expect(result.count).toBe(42n);
    expect(result.label).toBe("test");
  });

  test("serializeOutputs uses port type", () => {
    const nd = makeNodeData([], [
      { id: "1", name: "result", type: "string", optional: false },
      { id: "2", name: "count", type: "u64", optional: false },
    ]);
    const cmd = new BaseCommand(nd);
    const outputs = { result: "hello", count: 99 };
    const serialized = cmd.serializeOutputs(outputs);
    expect(serialized.result).toBeInstanceOf(Value);
    expect(serialized.result.S).toBe("hello");
    expect(serialized.count).toBeInstanceOf(Value);
    expect(serialized.count.U).toBe("99");
  });

  test("deserializeInputs falls back to toJSObject for unknown port", () => {
    const cmd = new BaseCommand(makeNodeData());
    const inputs = { unknown: Value.String("raw") };
    const result = cmd.deserializeInputs(inputs);
    expect(result.unknown).toBe("raw");
  });
});

describe("deserializeInput", () => {
  test("bool", () => {
    expect(deserializeInput({ type_bounds: ["bool"] }, Value.Boolean(true))).toBe(true);
  });

  test("u64 → bigint", () => {
    expect(deserializeInput({ type_bounds: ["u64"] }, Value.U64(100))).toBe(100n);
  });

  test("string", () => {
    expect(deserializeInput({ type_bounds: ["string"] }, Value.String("hi"))).toBe("hi");
  });

  test("f64 → number", () => {
    expect(deserializeInput({ type_bounds: ["f64"] }, Value.Float(1.5))).toBeCloseTo(1.5);
  });

  test("no matching type_bounds → undefined", () => {
    expect(deserializeInput({ type_bounds: ["free"] }, Value.String("x"))).toBeUndefined();
  });
});

describe("serializeOutput", () => {
  test("bool", () => {
    const v = serializeOutput({ type: "bool" }, true);
    expect(v?.B).toBe(true);
  });

  test("string", () => {
    const v = serializeOutput({ type: "string" }, "result");
    expect(v?.S).toBe("result");
  });

  test("u64", () => {
    const v = serializeOutput({ type: "u64" }, 500);
    expect(v?.U).toBe("500");
  });

  test("free → undefined", () => {
    expect(serializeOutput({ type: "free" }, "anything")).toBeUndefined();
  });
});
