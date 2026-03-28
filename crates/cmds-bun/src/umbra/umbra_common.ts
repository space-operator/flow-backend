/**
 * Shared helpers for all Umbra Privacy nodes.
 *
 * Supports two signing modes:
 * 1. Local keypair signing (createSignerFromPrivateKeyBytes)
 * 2. Framework wallet adapter signing (ctx.requestSignature)
 *
 * The SDK accepts dependency injection for both IUmbraSigner and
 * TransactionForwarder, which we use to integrate with the flow framework.
 */
import {
  getUmbraClientFromSigner,
  createSignerFromPrivateKeyBytes,
  getUmbraRelayer,
} from "@umbra-privacy/sdk";
import { PublicKey } from "@solana/web3.js";
import type { Context } from "@space-operator/flow-lib-bun";

export const INDEXER_ENDPOINT = "https://acqzie0a1h.execute-api.eu-central-1.amazonaws.com";
export const RELAYER_ENDPOINT = "https://6yn4ndrv2i.execute-api.eu-central-1.amazonaws.com";

const SUPPORTED_NETWORKS = ["mainnet", "devnet"] as const;

/**
 * Create an Umbra SDK client.
 *
 * Accepts either:
 * - 64-byte keypair: signs locally (works for all operations)
 * - 32-byte pubkey + ctx: signs via framework wallet adapter
 *   (works for deposit/withdraw, NOT for anonymous registration
 *    which requires signMessage)
 */
export async function createUmbraClient(
  keypairOrPubkey: Uint8Array,
  network: string,
  rpcUrl: string,
  ctx?: Context,
) {
  if (!SUPPORTED_NETWORKS.includes(network as any)) {
    throw new Error(
      `Umbra network "${network}" is not supported. Use "mainnet" or "devnet".`,
    );
  }

  const isKeypair = keypairOrPubkey.length === 64;
  const signer = isKeypair
    ? await createSignerFromPrivateKeyBytes(keypairOrPubkey)
    : createFlowSigner(ctx!, keypairOrPubkey);

  const rpcSubscriptionsUrl = rpcUrl
    .replace(/^https:\/\//, "wss://")
    .replace(/^http:\/\//, "ws://");
  const indexerApiEndpoint =
    network === "mainnet" ? INDEXER_ENDPOINT : undefined;

  const transactionForwarder = ctx
    ? createFrameworkForwarder(ctx)
    : undefined;

  return getUmbraClientFromSigner(
    {
      signer,
      network,
      rpcUrl,
      rpcSubscriptionsUrl,
      indexerApiEndpoint,
    } as any,
    transactionForwarder ? { transactionForwarder } as any : undefined,
  );
}

/**
 * Create an IUmbraSigner that delegates transaction signing to
 * the flow framework's wallet adapter via ctx.requestSignature().
 *
 * Kit v2 transaction.messageBytes is the same wire format as
 * web3.js v1 Message.serialize(), so they are directly compatible.
 *
 * signMessage is NOT supported (throws) because the framework
 * only has requestSignature for transaction messages. Operations
 * that need signMessage (anonymous registration) require a keypair.
 */
function createFlowSigner(ctx: Context, pubkeyBytes: Uint8Array) {
  const pubkey = new PublicKey(pubkeyBytes);
  const address = pubkey.toBase58();

  return {
    address,

    async signTransaction(transaction: any) {
      // transaction = { messageBytes: Uint8Array, signatures: Record<Address, SignatureBytes> }
      const messageBytes: Uint8Array = transaction.messageBytes;

      const { signature, new_message } = await ctx.requestSignature(
        pubkey,
        messageBytes,
      );

      // If wallet modified the message (e.g. Phantom adding priority fees),
      // update messageBytes accordingly
      const finalMessageBytes = new_message ?? messageBytes;

      // Insert the 64-byte Ed25519 signature into the kit v2 signatures map
      return {
        ...transaction,
        messageBytes: finalMessageBytes,
        signatures: {
          ...transaction.signatures,
          [address]: signature,
        },
      };
    },

    async signTransactions(transactions: any[]) {
      const results: any[] = [];
      for (const tx of transactions) {
        results.push(await this.signTransaction(tx));
      }
      return results;
    },

    async signMessage(_message: Uint8Array): Promise<never> {
      throw new Error(
        "signMessage is not supported with wallet adapter signing. " +
        "Operations that require signMessage (anonymous registration) " +
        "need a full keypair, not a pubkey-only wallet.",
      );
    },
  };
}

/**
 * Encode a @solana/kit v2 signed transaction into the Solana wire format.
 *
 * Wire format: [compact-u16 num_sigs, ...64-byte signatures, ...messageBytes]
 * The signatures map keys are ordered to match the message's signer order.
 */
function encodeWireTransaction(transaction: any): Buffer {
  const messageBytes: Uint8Array = transaction.messageBytes;
  const signatures: Record<string, Uint8Array | null> = transaction.signatures;

  const sigEntries = Object.values(signatures);
  const numSigs = sigEntries.length;

  // Compact-u16 encoding for num_sigs (< 128 = single byte)
  const header = numSigs < 0x80
    ? Buffer.from([numSigs])
    : Buffer.from([numSigs & 0x7f | 0x80, numSigs >> 7]);

  const sigBytes = Buffer.alloc(numSigs * 64);
  for (let i = 0; i < numSigs; i++) {
    const sig = sigEntries[i];
    if (sig) sigBytes.set(sig, i * 64);
    // null signatures stay as zeroes
  }

  return Buffer.concat([header, sigBytes, messageBytes]);
}

/**
 * Create a TransactionForwarder that submits signed transactions
 * through the framework's Solana connection.
 */
function createFrameworkForwarder(ctx: Context) {
  const sendViaFramework = async (transaction: any): Promise<string> => {
    const wireBytes = encodeWireTransaction(transaction);

    const signature = await ctx.solana.sendRawTransaction(wireBytes, {
      skipPreflight: false,
      preflightCommitment: "confirmed",
    });

    const latestBlockhash = await ctx.solana.getLatestBlockhash();
    await ctx.solana.confirmTransaction(
      { signature, ...latestBlockhash },
      "confirmed",
    );

    return signature;
  };

  return {
    forwardSequentially: async (transactions: any[], _options: any = {}) => {
      const signatures: string[] = [];
      for (const tx of transactions) {
        signatures.push(await sendViaFramework(tx));
      }
      return signatures;
    },
    forwardInParallel: async (transactions: any[], _options: any = {}) => {
      return Promise.all(transactions.map(sendViaFramework));
    },
  };
}

/**
 * Create the Umbra relayer client (needed for UTXO claims).
 */
export function createRelayer() {
  return getUmbraRelayer({ apiEndpoint: RELAYER_ENDPOINT });
}
