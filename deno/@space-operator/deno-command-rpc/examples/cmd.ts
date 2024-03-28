import { Context, Value } from "@space-operator/flow-lib";
import { CommandTrait, start } from "../src/mod.ts";

class Command implements CommandTrait {
  async run(
    ctx: Context,
    params: Record<string, Value>
  ): Promise<Record<string, Value>> {
    return {
      c: new Value(params["a"].toJSObject() + params["b"].toJSObject()),
    };
  }
}

await start(new Command());
