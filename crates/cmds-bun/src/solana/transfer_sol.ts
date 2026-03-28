import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { Instructions } from "@space-operator/flow-lib-bun/context";
import {
  Keypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";

export default class TransferSol extends BaseCommand {
  override async run(
    ctx: Context,
    inputs: {
      fee_payer?: Keypair;
      sender: Keypair;
      recipient: PublicKey;
      amount: number;
      submit?: boolean;
    },
  ): Promise<{ signature?: string }> {
    const senderKeypair = inputs.sender;
    const senderPubkey = senderKeypair.publicKey;
    const recipient = inputs.recipient;
    const lamports = inputs.amount;
    const submit = inputs.submit ?? true;

    console.log(
      `Transferring ${lamports} lamports to ${recipient.toBase58()}`,
    );

    const instruction = SystemProgram.transfer({
      fromPubkey: senderPubkey,
      toPubkey: recipient,
      lamports,
    });

    if (!submit) {
      return {};
    }

    // Build signers list, with optional separate fee payer
    const signers: Array<Keypair | PublicKey> = [];
    let feePayer = senderPubkey;

    if (inputs.fee_payer) {
      const feePayerKeypair = inputs.fee_payer;
      if (!feePayerKeypair.publicKey.equals(senderPubkey)) {
        feePayer = feePayerKeypair.publicKey;
        signers.push(feePayerKeypair);
      }
    }
    signers.push(senderKeypair);

    const instructions = new Instructions(feePayer, signers, [instruction]);

    const result = await ctx.execute(instructions, {});

    const signature = result.signature
      ? Buffer.from(result.signature).toString("base64")
      : undefined;

    console.log("Transfer complete:", signature);

    return { signature };
  }
}

// ── Tests (only run under `bun test`, safe to import elsewhere) ───────
import { test, expect, describe } from "bun:test";
try {
  describe("TransferSol", () => {
    test("build: class can be instantiated", () => {
      const nd = {
        type: "bun",
        node_id: "test",
        inputs: [],
        outputs: [],
        config: {},
      } as any;
      const cmd = new TransferSol(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });
  });
} catch (_) {
  // Not running under `bun test`
}
