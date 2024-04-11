import type { Context } from "./context.ts";

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
  /**
   * This function will be called every time the command is run.
   * @param ctx Context
   * @param params Map of input_name => input_value.
   */
  run(ctx: Context, params: Record<string, any>): Promise<Record<string, any>>;
}
