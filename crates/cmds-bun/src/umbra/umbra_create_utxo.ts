import { BaseCommand, Context } from "@space-operator/flow-lib-bun";
import {
  getUmbraClientFromSigner,
  createSignerFromPrivateKeyBytes,
  getCreateReceiverClaimableUtxoFromPublicBalanceFunction,
} from "@umbra-privacy/sdk";

async function createUmbraClient(keypairBytes: Uint8Array, network: string, rpcUrl: string) {
  const signer = await createSignerFromPrivateKeyBytes(keypairBytes);
  const rpcSubscriptionsUrl = rpcUrl.replace(/^https:\/\//, "wss://").replace(/^http:\/\//, "ws://");
  const indexerApiEndpoint = network === "mainnet" ? "https://acqzie0a1h.execute-api.eu-central-1.amazonaws.com" : undefined;
  return getUmbraClientFromSigner({ signer, network, rpcUrl, rpcSubscriptionsUrl, indexerApiEndpoint } as any);
}

export default class UmbraCreateUtxo extends BaseCommand {
  override async run(_ctx: Context, inputs: any): Promise<any> {
    const client = await createUmbraClient(
      new Uint8Array(inputs.keypair),
      inputs.network,
      inputs.rpc_url,
    );

    const createUtxo = (getCreateReceiverClaimableUtxoFromPublicBalanceFunction as any)({ client });

    // SDK signature: (args: CreateUtxoArgs, options?) => Promise<TransactionSignature[]>
    // CreateUtxoArgs = { amount: U64, destinationAddress: Address, mint: Address }
    const args = {
      amount: BigInt(inputs.amount) as any,         // U64 branded type
      destinationAddress: inputs.receiver as any,   // Address branded type
      mint: inputs.mint as any,                     // Address branded type
    };

    console.log(`Creating receiver-claimable UTXO in mixer pool...`);
    console.log(`  receiver: ${inputs.receiver}`);
    console.log(`  mint: ${inputs.mint}`);
    console.log(`  amount: ${args.amount}`);

    const signatures = await createUtxo(args);

    console.log(`UTXO created with ${signatures.length} transaction(s)`);
    signatures.forEach((sig: any, i: number) => console.log(`  tx ${i}: ${sig}`));

    return {
      signature: signatures.map(String).join(","),
    };
  }
}

// ── Tests ──────────────────────────────────────────────────────────────
import { test, expect, describe } from "bun:test";

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
