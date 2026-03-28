import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import { getUserRegistrationFunction } from "@umbra-privacy/sdk";
import { getUserRegistrationProver } from "@umbra-privacy/web-zk-prover";
import { createUmbraClient } from "./umbra_common.ts";

export default class UmbraRegister extends BaseCommand {
  override async run(ctx: Context, inputs: any): Promise<any> {
    const client = await createUmbraClient(
      new Uint8Array(inputs.keypair),
      inputs.network,
      inputs.rpc_url,
      ctx,
    );

    const confidential = inputs.confidential !== undefined ? inputs.confidential : true;
    const anonymous = inputs.anonymous !== undefined ? inputs.anonymous : true;

    const zkProver = anonymous ? getUserRegistrationProver() : undefined;
    const register = getUserRegistrationFunction({ client }, { zkProver } as any);

    console.log(`Registering Umbra user on ${inputs.network}...`);
    console.log(`  confidential: ${confidential}, anonymous: ${anonymous}`);

    const result = await register({ confidential, anonymous });

    console.log("Registration complete:", JSON.stringify(result, null, 2));

    return {
      signature: typeof result === "string" ? result : JSON.stringify(result),
    };
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
