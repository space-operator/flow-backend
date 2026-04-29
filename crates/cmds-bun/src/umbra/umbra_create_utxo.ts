import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  getPublicBalanceToReceiverClaimableUtxoCreatorFunction,
  getUserRegistrationFunction,
} from "@umbra-privacy/sdk";
import {
  createUmbraClient,
  createRustProver,
  getPrimarySignature,
  logUmbraError,
  resolveUmbraFeePayerBytes,
  resolveUmbraSignerBytes,
  safeJsonStringify,
  wrapZkProver,
} from "./umbra_common.ts";

function createRegistrationCallbacks() {
  const stepLogger = (step: string) => ({
    pre: async (ctx: any) => {
      console.log(`[create_utxo:register] phase: ${step}_start skipped=${Boolean(ctx?.skipped)}`);
    },
    post: async (ctx: any) => {
      const signature = typeof ctx?.signature === "string" ? ` signature=${ctx.signature}` : "";
      console.log(
        `[create_utxo:register] phase: ${step}_complete skipped=${Boolean(ctx?.skipped)}${signature}`,
      );
    },
  });

  return {
    userAccountInitialisation: stepLogger("user_account_initialisation"),
    registerX25519PublicKey: stepLogger("register_x25519_public_key"),
    registerUserForAnonymousUsage: stepLogger("register_user_for_anonymous_usage"),
  };
}

export default class UmbraCreateUtxo extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    console.log("[create_utxo] phase: client_creation");
    const client = await createUmbraClient(
      resolveUmbraSignerBytes(inputs),
      inputs.network,
      inputs.rpc_url,
      ctx,
      resolveUmbraFeePayerBytes(inputs),
    );

    if (inputs.ensure_registered !== false) {
      console.log("[create_utxo] phase: registration_prover_init");
      const registrationProver = wrapZkProver(
        "create_utxo:register",
        createRustProver("userRegistration"),
      );
      const register = getUserRegistrationFunction(
        { client },
        { zkProver: registrationProver } as any,
      );

      try {
        console.log("[create_utxo] phase: ensure_registration");
        await register({
          confidential: true,
          anonymous: true,
          callbacks: createRegistrationCallbacks(),
        } as any);
        console.log("[create_utxo] phase: ensure_registration_complete");
      } catch (err: any) {
        const details = logUmbraError("create_utxo:register", err);
        throw new Error(
          `Umbra registration failed (${details.phase}): ${details.message}${details.cause ? ` — cause: ${details.cause}` : ""}`,
        );
      }
    }

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
