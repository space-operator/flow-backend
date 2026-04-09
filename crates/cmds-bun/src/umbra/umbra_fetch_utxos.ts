import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getClaimableUtxoScannerFunction } from "@umbra-privacy/sdk";
import { createUmbraClient } from "./umbra_common.ts";

export default class UmbraFetchUtxos extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    if (!inputs.keypair || !inputs.network || !inputs.rpc_url) {
      throw new Error("Missing required inputs: keypair, network, rpc_url");
    }
    if (inputs.network !== "mainnet") {
      console.warn(`No public indexer available for ${inputs.network}. Umbra indexer is mainnet-only.`);
      console.warn(`  See: https://sdk.umbraprivacy.com/indexer/overview`);
      return { utxos: [], count: 0, error: `No indexer available for ${inputs.network}` };
    }

    const client = await createUmbraClient(
      new Uint8Array(inputs.keypair),
      inputs.network,
      inputs.rpc_url,
      ctx,
    );

    const fetchUtxos = getClaimableUtxoScannerFunction({ client });

    const treeIndex = inputs.tree_index !== undefined ? Number(inputs.tree_index) : 0;
    const startIndex = inputs.start_index !== undefined ? Number(inputs.start_index) : 0;
    const endIndex = inputs.end_index !== undefined ? Number(inputs.end_index) : undefined;

    console.log(`Fetching claimable UTXOs from indexer...`);
    console.log(`  tree_index: ${treeIndex}, start_index: ${startIndex}`);

    let result: any;
    try {
      result = endIndex !== undefined
        ? await fetchUtxos(treeIndex, startIndex, endIndex)
        : await fetchUtxos(treeIndex, startIndex);
    } catch (err: any) {
      const msg = err?.message ?? String(err);
      if (msg.includes("Unable to connect") || msg.includes("Network error") || msg.includes("ENOTFOUND")) {
        throw new Error(`Umbra indexer is unreachable. The service may be down. Details: ${msg}`);
      }
      throw err;
    }

    const ephemeral = result?.ephemeral ?? [];
    const receiver = result?.receiver ?? [];
    const allUtxos = [...ephemeral, ...receiver];

    console.log(`Found ${allUtxos.length} claimable UTXO(s) (${ephemeral.length} ephemeral, ${receiver.length} receiver)`);

    return { utxos: allUtxos, count: allUtxos.length };
  }
}

// ── Tests (only run under `bun test`, safe to import elsewhere) ───────
import { test, expect, describe } from "bun:test";
try {
  describe("UmbraFetchUtxos", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraFetchUtxos(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("run: rejects with missing inputs", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraFetchUtxos(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow();
    });
  });
} catch (_) {
  // Not running under `bun test`
}
