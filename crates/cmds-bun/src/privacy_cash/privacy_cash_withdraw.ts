import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { createPrivacyCashClient } from "./privacy_cash_common.ts";

export default class PrivacyCashWithdraw extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const client = createPrivacyCashClient(
      inputs.keypair,
      inputs.rpc_url,
    );

    const amountLamports = Number(inputs.amount);
    if (!Number.isFinite(amountLamports) || amountLamports <= 0) {
      throw new Error(`Invalid amount: ${inputs.amount}. Must be a positive number of lamports.`);
    }

    if (!inputs.recipient) {
      throw new Error("Missing required input: recipient (Base58 public key)");
    }

    console.log(`Withdrawing ${amountLamports} lamports from Privacy Cash...`);
    console.log(`  recipient: ${inputs.recipient}`);

    const result = await client.withdraw();

    console.log("Withdraw complete:", JSON.stringify(result));

    return {
      signature: result.tx ?? String(result),
    };
  }
}

// ── Tests ───────
import { test, expect, describe } from "bun:test";
try {
  describe("PrivacyCashWithdraw", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new PrivacyCashWithdraw(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("run: rejects with missing inputs", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new PrivacyCashWithdraw(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow();
    });
  });
} catch (_) {}
