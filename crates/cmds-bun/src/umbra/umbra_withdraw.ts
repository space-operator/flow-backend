import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getDirectWithdrawIntoPublicBalanceV3Function } from "@umbra-privacy/sdk";
import { createUmbraClient } from "./umbra_common.ts";

export default class UmbraWithdraw extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const client = await createUmbraClient(
      new Uint8Array(inputs.keypair),
      inputs.network,
      inputs.rpc_url,
      ctx,
    );

    const withdraw = getDirectWithdrawIntoPublicBalanceV3Function({ client });
    const amount = BigInt(inputs.amount) as any;
    const destination = (inputs.destination || client.signer.address) as any;

    console.log(`Withdrawing ${amount} tokens from encrypted balance...`);
    console.log(`  destination: ${destination}`);
    console.log(`  mint: ${inputs.mint}`);

    const result = await withdraw(destination, inputs.mint as any, amount);

    console.log("Withdrawal complete:", String(result));

    return { signature: String(result) };
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
