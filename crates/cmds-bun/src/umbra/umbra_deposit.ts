import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getDirectDepositIntoEncryptedBalanceFunction } from "@umbra-privacy/sdk";
import { createUmbraClient } from "./umbra_common.ts";

export default class UmbraDeposit extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const client = await createUmbraClient(
      new Uint8Array(inputs.keypair),
      inputs.network,
      inputs.rpc_url,
      ctx,
    );

    const deposit = getDirectDepositIntoEncryptedBalanceFunction({ client });
    const amount = BigInt(inputs.amount) as any;

    console.log(`Depositing ${amount} tokens into encrypted balance...`);
    console.log(`  destination: ${inputs.destination}`);
    console.log(`  mint: ${inputs.mint}`);

    const result = await deposit(
      inputs.destination as any,
      inputs.mint as any,
      amount,
    );

    console.log("Deposit complete:", String(result));

    return { signature: String(result) };
  }
}

// ── Tests (only run under `bun test`, safe to import elsewhere) ───────
import { test, expect, describe } from "bun:test";
try {
  describe("UmbraDeposit", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraDeposit(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("run: rejects with missing inputs", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraDeposit(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow();
    });
  });
} catch (_) {
  // Not running under `bun test`
}
