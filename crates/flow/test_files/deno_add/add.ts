import { Context, CommandTrait } from "jsr:@space-operator/flow-lib@0.5.0";

export default class MyCommand implements CommandTrait {
  async run(
    _: Context,
    params: Record<string, any>
  ): Promise<Record<string, any>> {
    return { c: params.a + params.b };
  }
}
