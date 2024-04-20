import type { Context } from "./context.ts";
import { Value } from "./mod.ts";

/**
 * To write a node, write a class that implements this interface and make it the default export of your module.
 *
 * ```ts
 * import { Context, CommandTrait } from "jsr:@space-operator/flow-lib";
 *
 * export default class MyCommand implements CommandTrait {
 *   async run(
 *   _: Context,
 *   params: Record<string, any>
 *  ): Promise<Record<string, any>> {
 *    return { c: params.a + params.b };
 *  }
 * }
 * ```
 */
export interface CommandTrait {
  deserializeInputs?(inputs: Record<string, Value>): Record<string, any>;
  serializeOutputs?(outputs: Record<string, any>): Record<string, Value>;
  /**
   * This function will be called every time the command is run.
   * @param ctx Context
   * @param params Map of input_name => input_value.
   */
  run(ctx: Context, params: Record<string, any>): Promise<Record<string, any>>;
}

export type CommandType = "native" | "deno" | "WASM" | "mock";

export type ValueTypeBound = string;

export interface Source {
  id: string;
  name: string;
  type: ValueTypeBound;
  optional: boolean;
}

export interface Target {
  id: string;
  name: string;
  type_bounds: ValueTypeBound[];
  required: boolean;
  passthrough: boolean;
}

export interface NodeData {
  type: CommandType;
  node_id: string;
  sources: Source[];
  targets: Target[];
  // targets_form: any;
}

function deserializeInput(port: Target, input: Value): any | undefined {
  for (const type of port.type_bounds) {
    switch (type) {
      case "bool":
        return input.asBool();
      case "u8":
        return input.asNumber();
      case "u16":
        return input.asNumber();
      case "u32":
        return input.asNumber();
      case "u64":
        return input.asBigInt();
      case "u128":
        return input.asBigInt();
      case "i8":
        return input.asNumber();
      case "i16":
        return input.asNumber();
      case "i32":
        return input.asNumber();
      case "i64":
        return input.asBigInt();
      case "i128":
        return input.asBigInt();
      case "f32":
        return input.asNumber();
      case "f64":
        return input.asNumber();
      case "number":
        return input.asNumber();
      case "decimal":
        return input.asNumber();
      case "pubkey":
        return input.asPubkey();
      case "address":
        return input.asString();
      case "keypair":
        return input.asKeypair();
      case "signature":
        return input.asBytes();
      case "string":
        return input.asString();
      case "bytes":
        return input.asBytes();
      case "array":
      case "object":
      case "json":
      case "free":
    }
  }
  return undefined;
}

function serializeOutput(port: Source, output: any): Value | undefined {
  switch (port.type) {
    case "bool":
      return Value.Boolean(Boolean(output));
    case "u8":
      return Value.U64(parseInt(output));
    case "u16":
      return Value.U64(parseInt(output));
    case "u32":
      return Value.U64(parseInt(output));
    case "u64":
      return Value.U64(BigInt(output));
    case "u128":
      return Value.U128(BigInt(output));
    case "i8":
      return Value.I64(parseInt(output));
    case "i16":
      return Value.I64(parseInt(output));
    case "i32":
      return Value.I64(parseInt(output));
    case "i64":
      return Value.I64(BigInt(output));
    case "i128":
      return Value.I128(BigInt(output));
    case "f32":
      return Value.Float(parseFloat(output));
    case "f64":
      return Value.Float(parseFloat(output));
    case "number":
      return Value.Decimal(output);
    case "decimal":
      return Value.Decimal(output);
    case "pubkey":
      return Value.PublicKey(output);
    case "address":
      return Value.String(output);
    case "keypair":
      return Value.Keypair(output);
    case "signature":
      return Value.Signature(output);
    case "string":
      return Value.String(output);
    case "bytes":
      return Value.Bytes(output);
    case "array":
    case "object":
    case "json":
    case "free":
  }

  return undefined;
}

export class BaseCommand implements CommandTrait {
  protected nd: NodeData;
  constructor(nd: NodeData) {
    this.nd = nd;
  }

  deserializeInputs(inputs: Record<string, Value>): Record<string, any> {
    return Object.fromEntries(
      Object.entries(inputs).map(([k, v]) => {
        const port = this.nd.targets.find((v) => v.name === k);
        if (port !== undefined) {
          const de = deserializeInput(port, v);
          if (de !== undefined) {
            return [k, de];
          }
          return [k, v.toJSObject()];
        } else {
          return [k, v.toJSObject()];
        }
      })
    );
  }

  serializeOutputs(outputs: Record<string, any>): Record<string, Value> {
    return Object.fromEntries(
      Object.entries(outputs).map(([k, v]) => {
        const port = this.nd.sources.find((v) => v.name === k);
        if (port !== undefined) {
          const ser = serializeOutput(port, v);
          if (ser !== undefined) {
            return [k, ser];
          } else {
            return [k, new Value(v)];
          }
        } else {
          return [k, new Value(v)];
        }
      })
    );
  }

  run(
    _ctx: Context,
    _params: Record<string, any>
  ): Promise<Record<string, any>> {
    throw new Error("unimplemented");
  }
}
