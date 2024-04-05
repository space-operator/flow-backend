import * as lib from "jsr:@space-operator/flow-lib";
import * as web3 from "npm:@solana/web3.js";

export default class TransferSol implements lib.CommandTrait {
  async run(
    ctx: lib.Context,
    params: Record<string, any>
  ): Promise<Record<string, any>> {
    const fromPubkey = new web3.PublicKey(params.from);
    let tx = new web3.Transaction()
      .add(
        web3.ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 1000 })
      )
      .add(
        web3.SystemProgram.transfer({
          fromPubkey,
          toPubkey: new web3.PublicKey(params.to),
          lamports: params.amount,
        })
      );
    const { blockhash, lastValidBlockHeight } =
      await lib.solana.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = fromPubkey;

    const { signature, new_message } = await ctx.requestSignature(
      fromPubkey,
      tx.serializeMessage()
    );
    if (new_message) {
      tx = web3.Transaction.from(new_message);
    }
    tx.addSignature(fromPubkey, signature);
    return {
      signature: new lib.Value(await ctx.solana.sendRawTransaction(tx.serialize())),
    };
  }
}
