import * as lib from "jsr:@space-operator/flow-lib";
import * as web3 from "npm:@solana/web3.js";
import { encodeBase58 } from "jsr:@std/encoding@^0.220.1/base58";

export default class TransferSol implements lib.CommandTrait {
  async run(
    ctx: lib.Context,
    params: Record<string, any>
  ): Promise<Record<string, any>> {
    const fromPubkey = new web3.PublicKey(params.from);

    // build the message
    const message = new web3.TransactionMessage({
      payerKey: fromPubkey,
      recentBlockhash: (await ctx.solana.getLatestBlockhash()).blockhash,
      instructions: [
        web3.ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 1000 }),
        web3.SystemProgram.transfer({
          fromPubkey,
          toPubkey: new web3.PublicKey(params.to),
          lamports: params.amount,
        }),
      ],
    }).compileToLegacyMessage();

    // request signature from user
    const { signature, new_message } = await ctx.requestSignature(
      fromPubkey,
      message.serialize()
    );

    // submit
    const tx = web3.Transaction.populate(
      new_message ? web3.Message.from(new_message) : message,
      [encodeBase58(signature)]
    );
    return {
      signature: await ctx.solana.sendRawTransaction(tx.serialize()),
    };
  }
}
