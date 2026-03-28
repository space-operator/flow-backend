import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  getUmbraClientFromSigner,
  createSignerFromPrivateKeyBytes,
  getQueryUserAccountFunction,
} from "@umbra-privacy/sdk";

async function createUmbraClient(keypairBytes: Uint8Array, network: string, rpcUrl: string) {
  const signer = await createSignerFromPrivateKeyBytes(keypairBytes);
  const rpcSubscriptionsUrl = rpcUrl.replace(/^https:\/\//, "wss://").replace(/^http:\/\//, "ws://");
  const indexerApiEndpoint = network === "mainnet" ? "https://acqzie0a1h.execute-api.eu-central-1.amazonaws.com" : undefined;
  return getUmbraClientFromSigner({ signer, network, rpcUrl, rpcSubscriptionsUrl, indexerApiEndpoint } as any);
}

export default class UmbraQueryAccount extends BaseCommand {
  override async run(_ctx: Context, inputs: any): Promise<any> {
    const client = await createUmbraClient(
      new Uint8Array(inputs.keypair),
      inputs.network,
      inputs.rpc_url,
    );

    const queryAccount = getQueryUserAccountFunction({ client });

    // SDK signature: (userAddress: Address) => Promise<QueryUserAccountResult>
    // Always requires an address — default to the signer's own
    const address = (inputs.address || client.signer.address) as any;

    console.log(`Querying Umbra user account: ${address}`);

    try {
      const result = await queryAccount(address);
      console.log("Query result:", JSON.stringify(result, null, 2));

      return { exists: true, account: result };
    } catch (error: any) {
      if (
        error.message?.includes("not found") ||
        error.message?.includes("does not exist") ||
        error.message?.includes("Account does not exist")
      ) {
        console.log("Account does not exist");
        return { exists: false, account: null };
      }
      throw error;
    }
  }
}

// ── Tests ──────────────────────────────────────────────────────────────
import { test, expect, describe } from "bun:test";

describe("UmbraQueryAccount", () => {
  test("build: class can be instantiated", () => {
    const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
    const cmd = new UmbraQueryAccount(nd);
    expect(cmd).toBeInstanceOf(BaseCommand);
    expect(cmd.run).toBeInstanceOf(Function);
  });

  test("run: rejects with missing inputs", async () => {
    const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
    const cmd = new UmbraQueryAccount(nd);
    const ctx = {} as Context;
    await expect(cmd.run(ctx, {})).rejects.toThrow();
  });
});
