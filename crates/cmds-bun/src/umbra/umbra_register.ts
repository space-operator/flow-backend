import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getUserRegistrationFunction } from "@umbra-privacy/sdk";
import {
  createUmbraClient,
  createRustProver,
  getPrimarySignature,
  logUmbraError,
  safeJsonStringify,
  wrapZkProver,
} from "./umbra_common.ts";

function createRegistrationCallbacks() {
  const stepLogger = (step: string) => ({
    pre: async (ctx: any) => {
      console.log(`[register] phase: ${step}_start skipped=${Boolean(ctx?.skipped)}`);
    },
    post: async (ctx: any) => {
      const signature = typeof ctx?.signature === "string" ? ` signature=${ctx.signature}` : "";
      console.log(
        `[register] phase: ${step}_complete skipped=${Boolean(ctx?.skipped)}${signature}`,
      );
    },
  });

  return {
    userAccountInitialisation: stepLogger("user_account_initialisation"),
    registerX25519PublicKey: stepLogger("register_x25519_public_key"),
    registerUserForAnonymousUsage: stepLogger("register_user_for_anonymous_usage"),
  };
}

export default class UmbraRegister extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    console.log("[register] phase: client_creation");
    const client = await createUmbraClient(
      new Uint8Array(inputs.keypair),
      inputs.network,
      inputs.rpc_url,
      ctx,
    );

    const confidential = inputs.confidential !== undefined ? inputs.confidential : true;
    const anonymous = inputs.anonymous !== undefined ? inputs.anonymous : true;

    console.log("[register] phase: prover_init");
    const zkProver = anonymous
      ? wrapZkProver("register", createRustProver("userRegistration"))
      : undefined;

    console.log("[register] phase: function_creation");
    const register = getUserRegistrationFunction({ client }, zkProver ? { zkProver } as any : undefined);

    console.log(`Registering Umbra user on ${inputs.network}...`);
    console.log(`  confidential: ${confidential}, anonymous: ${anonymous}`);
    if (!anonymous) {
      console.warn(
        "[register] Umbra mixer docs require anonymous=true registration before UTXO mixer flows.",
      );
    }

    try {
      console.log("[register] phase: execution");
      const result = await register({
        confidential,
        anonymous,
        callbacks: createRegistrationCallbacks(),
      } as any);

      console.log("Registration result type:", typeof result, Array.isArray(result) ? `(array len=${result.length})` : "");
      console.log("Registration complete:", safeJsonStringify(result, 2));

      return { signature: getPrimarySignature(result) };
    } catch (err: any) {
      const details = logUmbraError("register", err);
      throw new Error(
        `Registration failed (${details.phase}): ${details.message}${details.cause ? ` — cause: ${details.cause}` : ""}`,
      );
    }
  }
}

// ── Tests (only run under `bun test`, safe to import elsewhere) ───────
import { test, expect, describe } from "bun:test";
try {
  describe("UmbraRegister", () => {
    test("build: class can be instantiated", () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraRegister(nd);
      expect(cmd).toBeInstanceOf(BaseCommand);
      expect(cmd.run).toBeInstanceOf(Function);
    });

    test("run: rejects with missing inputs", async () => {
      const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
      const cmd = new UmbraRegister(nd);
      const ctx = {} as Context;
      await expect(cmd.run(ctx, {})).rejects.toThrow();
    });
  });
} catch (_) {
  // Not running under `bun test`
}
