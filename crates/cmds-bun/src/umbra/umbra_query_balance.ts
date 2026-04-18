import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getEncryptedBalanceQuerierFunction } from "@umbra-privacy/sdk";
import { createUmbraClient, resolveUmbraSignerBytes } from "./umbra_common.ts";

export default class UmbraQueryBalance extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const client = await createUmbraClient(
      resolveUmbraSignerBytes(inputs),
      inputs.network,
      inputs.rpc_url,
      ctx,
    );

    const queryBalance = getEncryptedBalanceQuerierFunction({ client });
    const mints = [inputs.mint as any];

    console.log(`Querying encrypted balance for mint: ${inputs.mint}`);

    const resultMap = await queryBalance(mints);

    const entry: any = resultMap.get(inputs.mint as any);
    const balance = entry?.balance !== undefined
      ? entry.balance.toString()
      : "0";

    // Convert Map to serializable object
    const result: any = {};
    for (const [key, value] of resultMap.entries()) {
      result[String(key)] = value;
    }

    console.log("Balance result:", JSON.stringify(result, (_k: string, v: any) => typeof v === "bigint" ? v.toString() : v, 2));

    // Ensure result is BigInt-safe for JSON serialization
    const safeResult = JSON.parse(JSON.stringify(result, (_k: string, v: any) => typeof v === "bigint" ? v.toString() : v));
    return { balance, result: safeResult };
  }
}

// ── Tests (only run under `bun test`, safe to import elsewhere) ───────
import { test, expect, describe } from "bun:test";
try {
  describe("UmbraQueryBalance", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraQueryBalance(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("run: rejects with missing inputs", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraQueryBalance(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow();
    });
  });
} catch (_) {
  // Not running under `bun test`
}
