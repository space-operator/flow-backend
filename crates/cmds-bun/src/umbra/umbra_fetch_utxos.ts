import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getClaimableUtxoScannerFunction } from "@umbra-privacy/sdk";
import { createUmbraClient, resolveUmbraSignerBytes } from "./umbra_common.ts";

export default class UmbraFetchUtxos extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    if (!inputs.network || !inputs.rpc_url) {
      throw new Error("Missing required inputs: network, rpc_url");
    }

    const client = await createUmbraClient(
      resolveUmbraSignerBytes(inputs),
      inputs.network,
      inputs.rpc_url,
      ctx,
    );

    const fetchUtxos = getClaimableUtxoScannerFunction({ client });

    const treeIndex = inputs.tree_index !== undefined ? BigInt(inputs.tree_index) : 0n;
    const startIndex = inputs.start_index !== undefined ? BigInt(inputs.start_index) : 0n;
    const endIndex = inputs.end_index !== undefined ? BigInt(inputs.end_index) : undefined;

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

    // SDK returns { selfBurnable, received, publicSelfBurnable, publicReceived, nextScanStartIndex }
    const selfBurnable = result?.selfBurnable ?? [];
    const received = result?.received ?? [];
    const publicSelfBurnable = result?.publicSelfBurnable ?? [];
    const publicReceived = result?.publicReceived ?? [];

    // Public UTXOs carry a plaintext destinationAddress. Filter public lists to
    // only those addressed to our signer for safe downstream use.
    const signerAddress = String(client.signer.address);
    const mineOnly = (u: any) =>
      !u?.destinationAddress || String(u.destinationAddress) === signerAddress;
    const myPublicSelfBurnable = publicSelfBurnable.filter(mineOnly);
    const myPublicReceived = publicReceived.filter(mineOnly);

    // `utxos` feeds umbra_claim_utxo, which is wired to the SDK's
    // getReceiverClaimableUtxoToEncryptedBalanceClaimerFunction. That claimer
    // consumes the scanner's `received` category (encrypted receiver-claimable
    // UTXOs). Public-tagged UTXOs have a different on-chain shape and make
    // the claimer throw byte-size errors, so we expose them via separate
    // outputs instead and keep `utxos` restricted to what Claim can process.
    //
    // SDK 4.0 note: as of @umbra-privacy/sdk@4.0.0, both
    // getPublicBalanceToReceiverClaimableUtxoCreatorFunction AND
    // getEncryptedBalanceToReceiverClaimableUtxoCreatorFunction round-trip
    // through the indexer as `public-received` (not `received`). Until that
    // upstream behavior changes, end-to-end Create→Fetch→Claim of
    // receiver-claimable UTXOs on devnet will see the UTXO land in
    // `publicReceived` and `utxos` stay empty. See docs/issues.md.
    const claimableByReceiver = [...received];
    const allUtxos = [
      ...selfBurnable,
      ...received,
      ...myPublicSelfBurnable,
      ...myPublicReceived,
    ];

    console.log(
      `Found ${allUtxos.length} UTXO(s) total — ` +
        `claimable by Receiver claim: ${claimableByReceiver.length} ` +
        `(selfBurnable=${selfBurnable.length}, received=${received.length}, ` +
        `publicSelfBurnable=${myPublicSelfBurnable.length}, ` +
        `publicReceived=${myPublicReceived.length})`,
    );

    return {
      utxos: claimableByReceiver,
      count: claimableByReceiver.length,
      allUtxos,
      publicReceived: myPublicReceived,
      publicSelfBurnable: myPublicSelfBurnable,
      selfBurnable,
      received,
      nextScanStartIndex: result?.nextScanStartIndex,
    };
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
