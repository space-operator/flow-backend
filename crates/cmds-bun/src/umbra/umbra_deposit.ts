import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  getUmbraClientFromSigner,
  createSignerFromPrivateKeyBytes,
  getDirectDepositIntoEncryptedBalanceFunction,
} from "@umbra-privacy/sdk";

async function createUmbraClient(keypairBytes: Uint8Array, network: string, rpcUrl: string) {
  const signer = await createSignerFromPrivateKeyBytes(keypairBytes);
  const rpcSubscriptionsUrl = rpcUrl.replace(/^https:\/\//, "wss://").replace(/^http:\/\//, "ws://");
  const indexerApiEndpoint = network === "mainnet" ? "https://acqzie0a1h.execute-api.eu-central-1.amazonaws.com" : undefined;
  return getUmbraClientFromSigner({ signer, network, rpcUrl, rpcSubscriptionsUrl, indexerApiEndpoint } as any);
}

export default class UmbraDeposit extends BaseCommand {
  override async run(_ctx: Context, inputs: any): Promise<any> {
    const client = await createUmbraClient(
      new Uint8Array(inputs.keypair),
      inputs.network,
      inputs.rpc_url,
    );

    const deposit = getDirectDepositIntoEncryptedBalanceFunction({ client });

    // SDK signature: (destinationAddress: Address, mint: Address, transferAmount: U64)
    const amount = BigInt(inputs.amount) as any; // U64 branded type

    console.log(`Depositing ${amount} tokens into encrypted balance...`);
    console.log(`  destination: ${inputs.destination}`);
    console.log(`  mint: ${inputs.mint}`);

    const result = await deposit(
      inputs.destination as any,
      inputs.mint as any,
      amount,
    );

    console.log("Deposit complete:", String(result));

    return { signature: String(result) };
  }
}

// ── Tests ──────────────────────────────────────────────────────────────
import { test, expect, describe } from "bun:test";

describe("UmbraDeposit", () => {
  test("build: class can be instantiated", () => {
    const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
    const cmd = new UmbraDeposit(nd);
    expect(cmd).toBeInstanceOf(BaseCommand);
    expect(cmd.run).toBeInstanceOf(Function);
  });

  test("run: rejects with missing inputs", async () => {
    const nd = { type: "bun", node_id: "test", inputs: [], outputs: [], config: {} } as any;
    const cmd = new UmbraDeposit(nd);
    const ctx = {} as Context;
    await expect(cmd.run(ctx, {})).rejects.toThrow();
  });
});
