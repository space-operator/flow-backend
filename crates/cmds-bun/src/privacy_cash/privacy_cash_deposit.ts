import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { createPrivacyCashClient } from "./privacy_cash_common.ts";

export default class PrivacyCashDeposit extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const client = createPrivacyCashClient(
      inputs.keypair,
      inputs.rpc_url,
    );

    const amountLamports = Number(inputs.amount);
    if (!Number.isFinite(amountLamports) || amountLamports <= 0) {
      throw new Error(`Invalid amount: ${inputs.amount}. Must be a positive number of lamports.`);
    }

    console.log(`Depositing ${amountLamports} lamports into Privacy Cash...`);

    const result = await client.deposit();

    console.log("Deposit complete:", JSON.stringify(result));

    return {
      signature: result.tx ?? String(result),
    };
  }
}

// ── Tests (only run under `bun test`, safe to import elsewhere) ───────
import { test, expect, describe } from "bun:test";
try {
  describe("PrivacyCashDeposit", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new PrivacyCashDeposit(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("run: rejects with missing inputs", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new PrivacyCashDeposit(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow();
    });
  });
} catch (_) {
  // Not running under `bun test`
}
