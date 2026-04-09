import { BaseCommand, Context } from "@space-operator/flow-lib-bun";

export default class Add extends BaseCommand {
  override async run(_ctx: Context, inputs: { a: number; b: number }): Promise<{ c: number }> {
    return { c: inputs.a + inputs.b };
  }
}
