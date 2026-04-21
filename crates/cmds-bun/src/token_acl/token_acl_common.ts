/**
 * Shared helpers for Token ACL (sRFC37) bun nodes.
 *
 * Bridges the framework's `@solana/web3.js` v1 Keypair I/O to the new-gen
 * `@solana/kit` runtime that `@solana/token-acl-sdk` is built on.
 *
 * The SDK expects:
 *   - `TransactionSigner` (kit) instead of `Keypair` (web3.js v1)
 *   - `Address` (branded string) instead of `PublicKey`
 *   - Pipe-based tx message construction + `signAndSendTransactionMessageWithSigners`
 */
import {
  address,
  createKeyPairSignerFromBytes,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  sendAndConfirmTransactionFactory,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstructions,
  getSignatureFromTransaction,
  getSignersFromTransactionMessage,
  signTransactionMessageWithSigners,
  type Address,
  type TransactionSigner,
  type TransactionModifyingSigner,
  type Instruction,
  type Rpc,
  type SolanaRpcApi,
  type Transaction,
  type TransactionWithinSizeLimit,
  type TransactionWithLifetime,
} from "@solana/kit";
import type { Context } from "@space-operator/flow-lib-bun";
import { Keypair, PublicKey } from "@solana/web3.js";

/** Canonical Token ACL program (mainnet + devnet) */
export const TOKEN_ACL_PROGRAM_ID = "TACLkU6CiCdkQN2MjoyDkVg2yAH9zkxiHDsiztQ52TP";
/** Canonical ABL Gate Program (reference "always-allow / allow-block-list" gate) */
export const ABL_GATE_PROGRAM_ID = "GATEzzqxhJnsWF6vHRsgtixxSB8PaQdcqGEVTEHWiULz";

/**
 * Normalise whatever keypair shape the framework handed us into raw 64-byte
 * secret key bytes.
 */
function toSecretKeyBytes(keypairInput: any): Uint8Array {
  if (keypairInput instanceof Uint8Array && keypairInput.length === 64) {
    return keypairInput;
  }
  if (keypairInput instanceof Keypair) {
    return keypairInput.secretKey;
  }
  if (keypairInput?.secretKey) {
    return new Uint8Array(keypairInput.secretKey);
  }
  if (keypairInput?._keypair?.secretKey) {
    return new Uint8Array(keypairInput._keypair.secretKey);
  }
  const directIndexed = extractIndexedBytes(keypairInput, 64);
  if (directIndexed) {
    return directIndexed;
  }
  if (keypairInput?.secretKey) {
    const nestedIndexed = extractIndexedBytes(keypairInput.secretKey, 64);
    if (nestedIndexed) {
      return nestedIndexed;
    }
  }
  throw new Error(
    `Cannot normalise keypair: ${typeof keypairInput}, keys=${
      keypairInput ? Object.keys(keypairInput).join(",") : "null"
    }`,
  );
}

type FlowAdapterTransactionSigner = TransactionModifyingSigner & {
  __flow_backend_adapter: true;
};

function isFlowAdapterTransactionSigner(
  signer: TransactionSigner,
): signer is FlowAdapterTransactionSigner {
  return (signer as { __flow_backend_adapter?: boolean }).__flow_backend_adapter ===
    true;
}

function extractAdapterWalletAddress(keypairInput: any): Address | null {
  if (!isRustWalletAdapterRecord(keypairInput)) {
    return null;
  }
  try {
    return toAddress(keypairInput.public_key);
  } catch {
    return null;
  }
}

function isRustWalletAdapterRecord(
  value: unknown,
): value is { public_key: unknown; token: string | null } {
  if (typeof value !== "object" || value === null) return false;
  const record = value as Record<string, unknown>;
  return "public_key" in record &&
    "token" in record &&
    (record.token === null || typeof record.token === "string");
}

function createAdapterTransactionSigner(
  ctx: Context,
  signerAddress: Address,
): FlowAdapterTransactionSigner {
  const publicKey = new PublicKey(signerAddress);
  return {
    __flow_backend_adapter: true,
    address: signerAddress,
    async modifyAndSignTransactions(
      transactions: readonly (
        | Transaction
        | (Transaction & TransactionWithLifetime)
      )[],
    ): Promise<
      readonly (Transaction & TransactionWithinSizeLimit & TransactionWithLifetime)[]
    > {
      return await Promise.all(
        transactions.map(async (transaction) => {
          const { signature, new_message } = await ctx.requestSignature(
            publicKey,
            new Uint8Array(transaction.messageBytes),
          );
          return {
            ...transaction,
            messageBytes: (new_message ?? transaction.messageBytes) as typeof transaction.messageBytes,
            signatures: {
              ...transaction.signatures,
              [signerAddress]: signature,
            },
          } as Transaction & TransactionWithinSizeLimit & TransactionWithLifetime;
        }),
      );
    },
  };
}

/**
 * Convert raw 64-byte secret → `@solana/kit` `TransactionSigner`.
 *
 * `@solana/kit`'s tx builder requires a *single* signer instance per address;
 * passing two separately-constructed signers for the same pubkey raises
 * "Multiple distinct signers were identified for address". We dedupe against
 * `cache` (pass the same Map across multiple `toKitSigner` calls within a
 * single node run) so `fee_payer === authority` wiring works naturally.
 */
export async function toKitSigner(
  ctx: Context,
  keypairInput: any,
  cache?: Map<string, TransactionSigner>,
): Promise<TransactionSigner> {
  let signer: TransactionSigner;
  const adapterAddress = extractAdapterWalletAddress(keypairInput);
  if (adapterAddress) {
    signer = createAdapterTransactionSigner(ctx, adapterAddress);
  } else {
    const bytes = toSecretKeyBytes(keypairInput);
    signer = await createKeyPairSignerFromBytes(bytes);
  }
  if (!cache) return signer;
  const existing = cache.get(signer.address);
  if (existing) {
    if (isFlowAdapterTransactionSigner(existing) &&
      !isFlowAdapterTransactionSigner(signer)) {
      cache.set(signer.address, signer);
      return signer;
    }
    return existing;
  }
  cache.set(signer.address, signer);
  return signer;
}

/** Fresh signer cache for one node run — pass to every `toKitSigner` call. */
export function newSignerCache(): Map<string, TransactionSigner> {
  return new Map();
}

/**
 * Build RPC + subscription handles from an `rpc_url`.
 *
 * Subscription URL is derived by swapping http(s) → ws(s); callers may pass an
 * explicit `wsUrl` to override.
 */
export function createRpc(
  rpcUrl: string,
  wsUrl?: string,
): {
  rpc: Rpc<SolanaRpcApi>;
  rpcSubscriptions: ReturnType<typeof createSolanaRpcSubscriptions>;
} {
  const ws =
    wsUrl ??
    rpcUrl.replace(/^http(s?):\/\//, (_m, s) => (s ? "wss://" : "ws://"));
  return {
    rpc: createSolanaRpc(rpcUrl),
    rpcSubscriptions: createSolanaRpcSubscriptions(ws),
  };
}

/**
 * Coerce a pubkey-ish input to kit `Address`.
 *
 * Handles:
 *   - plain base58 string
 *   - web3.js PublicKey-like (has `.toBase58()`)
 *   - IValue-wrapped strings (`{S: "..."}`, `{B3: "..."}` — the shapes the
 *     flow2 runtime uses when handing typed values to bun nodes).
 */
export function toAddress(v: any): Address {
  if (typeof v === "string") return address(v);
  if (v instanceof Uint8Array && v.length === 32) {
    return address(new PublicKey(v).toBase58());
  }
  if (Array.isArray(v) && v.length === 32) {
    return address(new PublicKey(v).toBase58());
  }
  if (v && typeof v === "object") {
    if (typeof v.S === "string") return address(v.S);
    if (typeof v.B3 === "string") return address(v.B3);
    if (typeof v.toBase58 === "function") return address(v.toBase58());
    const indexed = extractIndexedBytes(v as Record<string, unknown>, 32);
    if (indexed) return address(new PublicKey(indexed).toBase58());
  }
  throw new Error(
    `Cannot coerce to Address: ${typeof v} ${
      v && typeof v === "object" ? `keys=${Object.keys(v).join(",")}` : String(v)
    }`,
  );
}

/**
 * Build a single-instruction tx, sign with the provided signer, send, and
 * wait for confirmation. Returns the base58 signature string.
 */
export async function signAndSendSingle(
  rpcUrl: string,
  feePayer: TransactionSigner,
  instruction: Instruction,
): Promise<string> {
  return signAndSendMany(rpcUrl, feePayer, [instruction]);
}

export async function signAndSendMany(
  rpcUrl: string,
  feePayer: TransactionSigner,
  instructions: Instruction[],
): Promise<string> {
  const { rpc, rpcSubscriptions } = createRpc(rpcUrl);
  const { value: latestBlockhash } = await rpc
    .getLatestBlockhash({ commitment: "confirmed" })
    .send();

  const txMessage = pipe(
    createTransactionMessage({ version: 0 }),
    (m) => setTransactionMessageFeePayerSigner(feePayer, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
    (m) => appendTransactionMessageInstructions(instructions, m),
  );

  const adapterSignerAddresses = new Set(
    getSignersFromTransactionMessage(txMessage)
      .filter(isFlowAdapterTransactionSigner)
      .map((signer) => signer.address),
  );
  if (adapterSignerAddresses.size > 1) {
    throw new Error(
      "Token ACL bun nodes currently support at most one distinct Flow adapter wallet signer per transaction. Use a local keypair for the second signer, or make fee_payer and authority the same wallet.",
    );
  }

  const signedTx = await signTransactionMessageWithSigners(txMessage);
  const signature = getSignatureFromTransaction(signedTx);

  const sendAndConfirm = sendAndConfirmTransactionFactory({
    rpc,
    rpcSubscriptions,
  });
  await sendAndConfirm(signedTx, { commitment: "confirmed" });
  return signature;
}

function extractIndexedBytes(
  value: unknown,
  size: number,
): Uint8Array | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (!(String(0) in record) || !(String(size - 1) in record)) {
    return null;
  }
  const bytes = new Uint8Array(size);
  for (let i = 0; i < size; i++) {
    bytes[i] = Number(record[String(i)] ?? 0);
  }
  return bytes;
}

import { describe, expect, test } from "bun:test";
try {
  describe("token_acl_common", () => {
    test("toAddress: accepts indexed pubkey bytes", () => {
      const publicKey = Keypair.generate().publicKey;
      const indexed = Object.fromEntries(
        Array.from(publicKey.toBytes(), (value, index) => [String(index), value]),
      );
      expect(toAddress(indexed)).toBe(publicKey.toBase58());
    });

    test("toKitSigner: adapter wallets sign through ctx.requestSignature", async () => {
      const publicKey = Keypair.generate().publicKey;
      const signature = new Uint8Array(64).fill(7);
      const newMessage = new Uint8Array([9, 8, 7]);
      const signer = await toKitSigner(
        {
          async requestSignature() {
            return { signature, new_message: newMessage };
          },
        } as Context,
        { public_key: publicKey.toBytes(), token: null },
      );

      expect(isFlowAdapterTransactionSigner(signer)).toBe(true);
      const [signedTransaction] = await (signer as FlowAdapterTransactionSigner)
        .modifyAndSignTransactions([
          {
            messageBytes: new Uint8Array([1, 2, 3]),
            signatures: {},
          } as Transaction & TransactionWithLifetime,
        ]);

      expect(Array.from(signedTransaction.messageBytes)).toEqual(Array.from(newMessage));
      expect(Array.from(signedTransaction.signatures[publicKey.toBase58()]!)).toEqual(
        Array.from(signature),
      );
    });

    test("toKitSigner: does not treat non-Rust wallet-shaped objects as adapter wallets", async () => {
      await expect(
        toKitSigner(
          {} as Context,
          { publicKey: Keypair.generate().publicKey.toBytes() },
        ),
      ).rejects.toThrow("Cannot normalise keypair");
    });
  });
} catch (_) {
  // Not running under `bun test`
}
