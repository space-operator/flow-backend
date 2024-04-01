import * as rpc from "jsr:@space-operator/deno-command-rpc@^0.4.0";
import * as lib from "jsr:@space-operator/flow-lib@^0.4.0";
import * as web3 from "npm:@solana/web3.js";

export default class TransferSol implements rpc.CommandTrait {
  async run(
    ctx: lib.Context,
    params: Record<string, any>
  ): Promise<Record<string, any>> {
    const fromPubkey = new web3.PublicKey(params.from);
    let tx = new web3.Transaction().add(
      web3.SystemProgram.transfer({
        fromPubkey,
        toPubkey: new web3.PublicKey(params.to),
        lamports: params.amount,
      })
    );

    const { signature, new_message } = await ctx.requestSignature(
      fromPubkey,
      tx.serializeMessage()
    );
    if (new_message) {
      tx = web3.Transaction.from(new_message);
    }
    tx.addSignature(fromPubkey, signature);
    const client = new web3.Connection(ctx.cfg.solana_client.url);
    return { signature: await client.sendRawTransaction(tx.serialize()) };
  }
}
