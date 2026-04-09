import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getPublicBalanceToReceiverClaimableUtxoCreatorFunction } from "@umbra-privacy/sdk";
import {
  createUmbraClient,
  createRustProver,
  getPrimarySignature,
  logUmbraError,
  safeJsonStringify,
  wrapZkProver,
} from "./umbra_common.ts";

export default class UmbraCreateUtxo extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    console.log("[create_utxo] phase: client_creation");
    const client = await createUmbraClient(
      new Uint8Array(inputs.keypair),
      inputs.network,
      inputs.rpc_url,
      ctx,
    );

    console.log("[create_utxo] phase: prover_init");
    let zkProver: any;
    try {
      zkProver = wrapZkProver(
        "create_utxo",
        createRustProver("createDepositWithPublicAmount"),
      );
      console.log("[create_utxo] prover created:", typeof zkProver);
    } catch (err: any) {
      const details = logUmbraError("create_utxo:prover_init", err);
      throw new Error(`ZK prover initialization failed: ${details.message}`);
    }

    console.log("[create_utxo] phase: function_creation");
    const createUtxo = getPublicBalanceToReceiverClaimableUtxoCreatorFunction(
      { client },
      {
        zkProver,
        hooks: {
          createUtxo: {
            pre: async () => {
              console.log("[create_utxo] phase: deposit_transaction_sign_start");
            },
            post: async (_tx: any, signature: string) => {
              console.log(
                `[create_utxo] phase: deposit_transaction_sign_complete signature=${signature}`,
              );
            },
          },
        },
      } as any,
    );

    const args = {
      amount: BigInt(inputs.amount) as any,
      destinationAddress: inputs.receiver as any,
      mint: inputs.mint as any,
    };

    console.log(`[create_utxo] phase: execution`);
    console.log(`  receiver: ${inputs.receiver}`);
    console.log(`  mint: ${inputs.mint}`);
    console.log(`  amount: ${args.amount}`);
    console.log(
      "[create_utxo] Umbra mixer docs require sender and receiver to be fully registered with anonymous=true.",
    );

    try {
      const result = await (createUtxo as any)(args);

      console.log("[create_utxo] result type:", typeof result, Array.isArray(result) ? `(len=${result.length})` : "");
      console.log("[create_utxo] result:", safeJsonStringify(result, 2));

      return { signature: getPrimarySignature(result) };
    } catch (err: any) {
      const details = logUmbraError("create_utxo", err);
      throw new Error(
        `Create UTXO failed (${details.phase}): ${details.message}${details.cause ? ` — cause: ${details.cause}` : ""}`,
      );
    }
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
