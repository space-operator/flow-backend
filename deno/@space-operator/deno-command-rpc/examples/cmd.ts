import { type Context, Value } from "jsr:@space-operator/flow-lib";
import { CommandTrait } from "../src/mod.ts";

export default class Command implements CommandTrait {
  async run(
    ctx: Context,
    params: Record<string, Value>
  ): Promise<Record<string, Value>> {
    return {
      c: new Value(params["a"].toJSObject() + params["b"].toJSObject()),
    };
  }
}
