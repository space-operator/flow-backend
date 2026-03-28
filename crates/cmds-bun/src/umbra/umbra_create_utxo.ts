import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getCreateReceiverClaimableUtxoFromPublicBalanceFunction } from "@umbra-privacy/sdk";
import { getCreateReceiverClaimableUtxoFromPublicBalanceProver } from "@umbra-privacy/web-zk-prover";
import { createUmbraClient } from "./umbra_common.ts";

export default class UmbraCreateUtxo extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const client = await createUmbraClient(
      new Uint8Array(inputs.keypair),
      inputs.network,
      inputs.rpc_url,
      ctx,
    );

    const zkProver = getCreateReceiverClaimableUtxoFromPublicBalanceProver();
    const createUtxo = getCreateReceiverClaimableUtxoFromPublicBalanceFunction(
      { client },
      { zkProver } as any,
    );

    const args = {
      amount: BigInt(inputs.amount) as any,
      destinationAddress: inputs.receiver as any,
      mint: inputs.mint as any,
    };

    console.log(`Creating receiver-claimable UTXO in mixer pool...`);
    console.log(`  receiver: ${inputs.receiver}`);
    console.log(`  mint: ${inputs.mint}`);
    console.log(`  amount: ${args.amount}`);

    const signatures = await (createUtxo as any)(args);

    console.log(`UTXO created with ${signatures.length} transaction(s)`);
    signatures.forEach((sig: any, i: number) => console.log(`  tx ${i}: ${sig}`));

    return {
      signature: signatures.map(String).join(","),
    };
  }
}

// ── Tests (only run under `bun test`, safe to import elsewhere) ───────
import { test, expect, describe } from "bun:test";
try {
  describe("UmbraCreateUtxo", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraCreateUtxo(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("run: rejects with missing inputs", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraCreateUtxo(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow();
    });
  });
} catch (_) {
  // Not running under `bun test`
}
