import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  getUmbraClientFromSigner,
  createSignerFromPrivateKeyBytes,
  getFetchClaimableUtxosFunction,
} from "@umbra-privacy/sdk";

async function createUmbraClient(keypairBytes: Uint8Array, network: string, rpcUrl: string) {
  const signer = await createSignerFromPrivateKeyBytes(keypairBytes);
  const rpcSubscriptionsUrl = rpcUrl.replace(/^https:\/\//, "wss://").replace(/^http:\/\//, "ws://");
  const indexerApiEndpoint = network === "mainnet" ? "https://acqzie0a1h.execute-api.eu-central-1.amazonaws.com" : undefined;
  return getUmbraClientFromSigner({ signer, network, rpcUrl, rpcSubscriptionsUrl, indexerApiEndpoint } as any);
}

export default class UmbraFetchUtxos extends BaseCommand {
  override async run(_ctx: Context, inputs: any): Promise<any> {
    const client = await createUmbraClient(
      new Uint8Array(inputs.keypair),
      inputs.network,
      inputs.rpc_url,
    );

    if (inputs.network !== "mainnet") {
      console.warn(`⚠ No public indexer available for ${inputs.network}. Umbra indexer is mainnet-only.`);
      console.warn(`  See: https://sdk.umbraprivacy.com/indexer/overview`);
      return { utxos: [], count: 0, error: `No indexer available for ${inputs.network}` };
    }

    const fetchUtxos = getFetchClaimableUtxosFunction({ client });

    // SDK signature: (treeIndex: number, startInsertionIndex: number, endInsertionIndex?: number)
    //                => Promise<ClaimableUtxoResult>
    const treeIndex = inputs.tree_index !== undefined ? Number(inputs.tree_index) : 0;
    const startIndex = inputs.start_index !== undefined ? Number(inputs.start_index) : 0;
    const endIndex = inputs.end_index !== undefined ? Number(inputs.end_index) : undefined;

    console.log(`Fetching claimable UTXOs from indexer...`);
    console.log(`  tree_index: ${treeIndex}, start_index: ${startIndex}`);

    const result = endIndex !== undefined
      ? await fetchUtxos(treeIndex, startIndex, endIndex)
      : await fetchUtxos(treeIndex, startIndex);

    // ClaimableUtxoResult has .ephemeral (self-deposited) and .receiver (sent by others) arrays
    const ephemeral = (result as any)?.ephemeral ?? [];
    const receiver = (result as any)?.receiver ?? [];
    const allUtxos = [...ephemeral, ...receiver];

    console.log(`Found ${allUtxos.length} claimable UTXO(s) (${ephemeral.length} ephemeral, ${receiver.length} receiver)`);

    return { utxos: allUtxos, count: allUtxos.length };
  }
}

// ── Tests ──────────────────────────────────────────────────────────────
import { test, expect, describe } from "bun:test";

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
