import { Context, BaseCommand } from "jsr:@space-operator/flow-lib";

export default class MyCommand extends BaseCommand {
  async run(_: Context, inputs: any): Promise<any> {
    return { c: inputs.a + inputs.b };
  }
}
