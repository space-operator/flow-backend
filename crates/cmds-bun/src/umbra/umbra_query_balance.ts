import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  getUmbraClientFromSigner,
  createSignerFromPrivateKeyBytes,
  getQueryEncryptedBalanceFunction,
} from "@umbra-privacy/sdk";

async function createUmbraClient(keypairBytes: Uint8Array, network: string, rpcUrl: string) {
  const signer = await createSignerFromPrivateKeyBytes(keypairBytes);
  const rpcSubscriptionsUrl = rpcUrl.replace(/^https:\/\//, "wss://").replace(/^http:\/\//, "ws://");
  const indexerApiEndpoint = network === "mainnet" ? "https://acqzie0a1h.execute-api.eu-central-1.amazonaws.com" : undefined;
  return getUmbraClientFromSigner({ signer, network, rpcUrl, rpcSubscriptionsUrl, indexerApiEndpoint } as any);
}

export default class UmbraQueryBalance extends BaseCommand {
  override async run(_ctx: Context, inputs: any): Promise<any> {
    const client = await createUmbraClient(
      new Uint8Array(inputs.keypair),
      inputs.network,
      inputs.rpc_url,
    );

    const queryBalance = getQueryEncryptedBalanceFunction({ client });

    // SDK signature: (mints: readonly Address[]) => Promise<Map<Address, QueryEncryptedBalanceResult>>
    const mints = [inputs.mint as any]; // Wrap single mint into array

    console.log(`Querying encrypted balance for mint: ${inputs.mint}`);

    const resultMap = await queryBalance(mints);

    // Extract the single result from the Map
    const entry: any = resultMap.get(inputs.mint as any);
    const balance = entry?.balance !== undefined
      ? entry.balance.toString()
      : "0";

    // Convert Map to serializable object
    const result: any = {};
    for (const [key, value] of resultMap.entries()) {
      result[String(key)] = value;
    }

    console.log("Balance result:", JSON.stringify(result, null, 2));

    return { balance, result };
  }
}

// ── Tests ──────────────────────────────────────────────────────────────
import { test, expect, describe } from "bun:test";

describe("UmbraQueryBalance", () => {
  test("build: class can be instantiated", () => {
    const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
    const cmd = new UmbraQueryBalance(nd);
    expect(cmd).toBeInstanceOf(BaseCommand);
    expect(cmd.run).toBeInstanceOf(Function);
  });

  test("run: rejects with missing inputs", async () => {
    const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
    const cmd = new UmbraQueryBalance(nd);
    const ctx = {} as Context;
    await expect(cmd.run(ctx, {})).rejects.toThrow();
  });
});
