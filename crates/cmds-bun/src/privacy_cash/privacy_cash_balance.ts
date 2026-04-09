import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { createPrivacyCashClient } from "./privacy_cash_common.ts";

export default class PrivacyCashBalance extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const client = createPrivacyCashClient(
      inputs.keypair,
      inputs.rpc_url,
    );

    console.log("Querying Privacy Cash private balance...");

    const balance = await client.getPrivateBalance();

    console.log(`Private balance: ${balance} lamports`);

    return {
      balance: String(balance),
    };
  }
}

// ── Tests ───────
import { test, expect, describe } from "bun:test";
try {
  describe("PrivacyCashBalance", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new PrivacyCashBalance(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("run: rejects with missing inputs", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new PrivacyCashBalance(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow();
    });
  });
} catch (_) {}
