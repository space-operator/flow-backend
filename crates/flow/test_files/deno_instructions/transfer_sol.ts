import * as lib from "jsr:@space-operator/flow-lib";
import * as web3 from "npm:@solana/web3.js";
import { Instructions } from "jsr:@space-operator/flow-lib/context";

export default class TransferSol implements lib.CommandTrait {
  async run(
    ctx: lib.Context,
    params: Record<string, any>
  ): Promise<Record<string, any>> {
    const fromPubkey = new web3.PublicKey(params.from);

    const result = await ctx.execute(
      new Instructions(
        fromPubkey,
        [fromPubkey],
        [
          web3.SystemProgram.transfer({
            fromPubkey,
            toPubkey: new web3.PublicKey(params.to),
            lamports: params.amount,
          }),
        ]
      ),
      {}
    );

    return {
      signature: result.signature!,
    };
  }
}
