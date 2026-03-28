import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getClaimReceiverClaimableUtxoIntoEncryptedBalanceFunction } from "@umbra-privacy/sdk";
import { getClaimReceiverClaimableUtxoIntoEncryptedBalanceProver } from "@umbra-privacy/web-zk-prover";
import { createUmbraClient, createRelayer } from "./umbra_common.ts";

export default class UmbraClaimUtxo extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const client = await createUmbraClient(
      new Uint8Array(inputs.keypair),
      inputs.network,
      inputs.rpc_url,
      ctx,
    );

    const zkProver = getClaimReceiverClaimableUtxoIntoEncryptedBalanceProver();
    const relayer = createRelayer();
    const claimUtxo = getClaimReceiverClaimableUtxoIntoEncryptedBalanceFunction(
      { client },
      { zkProver, relayer } as any,
    );

    console.log(`Claiming UTXO into encrypted balance...`);
    console.log(`  utxo data: ${JSON.stringify(inputs.utxo_data).substring(0, 100)}...`);

    const result = await (claimUtxo as any)(inputs.utxo_data);

    console.log("UTXO claimed:", JSON.stringify(result, null, 2));

    return {
      signature: typeof result === "string" ? result : JSON.stringify(result),
    };
  }
}

// ── Tests (only run under `bun test`, safe to import elsewhere) ───────
import { test, expect, describe } from "bun:test";
try {
  describe("UmbraClaimUtxo", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraClaimUtxo(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("run: rejects with missing inputs", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraClaimUtxo(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow();
    });
  });
} catch (_) {
  // Not running under `bun test`
}
