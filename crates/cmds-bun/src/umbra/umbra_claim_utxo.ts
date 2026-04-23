import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getReceiverClaimableUtxoToEncryptedBalanceClaimerFunction } from "@umbra-privacy/sdk";
import {
  createRelayer,
  createUmbraClient,
  createRustProver,
  getPrimarySignature,
  logUmbraError,
  resolveUmbraSignerBytes,
  safeJsonStringify,
  wrapZkProver,
} from "./umbra_common.ts";

export default class UmbraClaimUtxo extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    console.log("[claim_utxo] phase: client_creation");
    const client = await createUmbraClient(
      resolveUmbraSignerBytes(inputs),
      inputs.network,
      inputs.rpc_url,
      ctx,
    );

    console.log("[claim_utxo] phase: prover_init");
    const zkProver = wrapZkProver(
      "claim_utxo",
      createRustProver("claimDepositIntoConfidentialAmount"),
    );
    const relayer = createRelayer(inputs.network);

    console.log("[claim_utxo] phase: function_creation");
    // SDK requires fetchBatchMerkleProof in deps. It's attached to the
    // client when indexerApiEndpoint is configured (see SDK index.js:784).
    const fetchBatchMerkleProof = (client as any).fetchBatchMerkleProof;
    if (!fetchBatchMerkleProof) {
      throw new Error(
        "Umbra client is missing fetchBatchMerkleProof — indexerApiEndpoint not configured for this network.",
      );
    }
    const claimUtxo = getReceiverClaimableUtxoToEncryptedBalanceClaimerFunction(
      { client },
      { zkProver, relayer, fetchBatchMerkleProof } as any,
    );

    console.log("[claim_utxo] phase: execution");
    console.log(`Claiming UTXO into encrypted balance...`);
    console.log(`  utxo data: ${safeJsonStringify(inputs.utxo_data).substring(0, 200)}...`);

    try {
      const result = await (claimUtxo as any)(inputs.utxo_data);

      console.log("UTXO claimed:", safeJsonStringify(result, 2));

      return {
        signature: getPrimarySignature(result),
      };
    } catch (err: any) {
      const details = logUmbraError("claim_utxo", err);
      throw new Error(
        `Claim UTXO failed (${details.phase}): ${details.message}${details.cause ? ` — cause: ${details.cause}` : ""}`,
      );
    }
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
