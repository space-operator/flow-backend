import { Context } from "jsr:@space-operator/flow-lib";
import { CommandTrait } from "jsr:@space-operator/deno-command-rpc@0.3.0";

export default class MyCommand implements CommandTrait {
  async run(
    _: Context,
    params: Record<string, any>
  ): Promise<Record<string, any>> {
    return { c: params.a + params.b };
  }
}
