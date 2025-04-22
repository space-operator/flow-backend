import { BaseCommand, Context } from "jsr:@space-operator/flow-lib@0.11.0";

export default class MyCommand extends BaseCommand {
  override async run(_: Context, inputs: any): Promise<any> {
    return { c: inputs.a + inputs.b };
  }
}
