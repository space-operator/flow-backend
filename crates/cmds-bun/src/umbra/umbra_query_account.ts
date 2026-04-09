import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getUserAccountQuerierFunction } from "@umbra-privacy/sdk";
import { createUmbraClient } from "./umbra_common.ts";

export default class UmbraQueryAccount extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const client = await createUmbraClient(
      new Uint8Array(inputs.keypair),
      inputs.network,
      inputs.rpc_url,
      ctx,
    );

    const queryAccount = getUserAccountQuerierFunction({ client });
    const address = (inputs.address || client.signer.address) as any;

    console.log(`Querying Umbra user account: ${address}`);

    const result = await queryAccount(address);
    console.log("Query result:", JSON.stringify(result, (_k, v) => typeof v === "bigint" ? v.toString() : v, 2));

    const exists = result?.state !== "non_existent";
    // Serialize BigInt values for JSON transport
    const account = exists ? JSON.parse(JSON.stringify(result, (_k, v) => typeof v === "bigint" ? v.toString() : v)) : null;
    return { exists, account };
  }
}

// ── Tests (only run under `bun test`, safe to import elsewhere) ───────
import { test, expect, describe } from "bun:test";
try {
  describe("UmbraQueryAccount", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraQueryAccount(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("run: rejects with missing inputs", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraQueryAccount(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow();
    });
  });
} catch (_) {
  // Not running under `bun test`
}
