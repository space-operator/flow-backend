import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getEncryptedBalanceToPublicBalanceDirectWithdrawerFunction } from "@umbra-privacy/sdk";
import {
  createUmbraClient,
  resolveUmbraFeePayerBytes,
  resolveUmbraSignerBytes,
} from "./umbra_common.ts";

export default class UmbraWithdraw extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const client = await createUmbraClient(
      resolveUmbraSignerBytes(inputs),
      inputs.network,
      inputs.rpc_url,
      ctx,
      resolveUmbraFeePayerBytes(inputs),
    );

    const withdraw = getEncryptedBalanceToPublicBalanceDirectWithdrawerFunction({ client });
    const amount = BigInt(inputs.amount);
    const chunkSize = inputs.chunk_size !== undefined
      ? BigInt(inputs.chunk_size)
      : 1_000_000n;
    const destination = (inputs.destination || client.signer.address) as any;

    console.log(`Withdrawing ${amount} tokens from encrypted balance...`);
    console.log(`  chunk_size: ${chunkSize}`);
    console.log(`  destination: ${destination}`);
    console.log(`  mint: ${inputs.mint}`);

    if (amount <= 0n) {
      throw new Error("amount must be greater than zero");
    }
    if (chunkSize <= 0n) {
      throw new Error("chunk_size must be greater than zero");
    }

    let remaining = amount;
    const results: any[] = [];
    while (remaining > 0n) {
      const nextAmount = remaining > chunkSize ? chunkSize : remaining;
      console.log(`  withdrawing chunk: ${nextAmount}`);
      const result = await withdraw(destination, inputs.mint as any, nextAmount as any);
      results.push({ amount: nextAmount, result });
      remaining -= nextAmount;
      console.log("Withdrawal chunk complete:", JSON.stringify(result, (_k: string, v: any) => typeof v === "bigint" ? v.toString() : v));
    }

    console.log("Withdrawal complete:", JSON.stringify(results, (_k: string, v: any) => typeof v === "bigint" ? v.toString() : v));

    const signature = results
      .map((entry) => {
        const result = entry.result;
        if (typeof result === "string") return result;
        if (Array.isArray(result)) return result.map(String).join(",");
        if (result && typeof result === "object") {
          return result.callbackSignature || result.queueSignature || result.signature || JSON.stringify(result, (_k: string, v: any) => typeof v === "bigint" ? v.toString() : v);
        }
        return String(result);
      })
      .filter(Boolean)
      .join(",");

    return {
      signature,
      chunks: results.map((entry) => ({
        amount: entry.amount.toString(),
        result: entry.result,
      })),
    };
  }
}

// ── Tests (only run under `bun test`, safe to import elsewhere) ───────
import { test, expect, describe } from "bun:test";
try {
  describe("UmbraWithdraw", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraWithdraw(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("run: rejects with missing inputs", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraWithdraw(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow();
    });
  });
} catch (_) {
  // Not running under `bun test`
}
