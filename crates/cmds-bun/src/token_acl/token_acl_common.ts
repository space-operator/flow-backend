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
  signTransactionMessageWithSigners,
  type Address,
  type TransactionSigner,
  type Instruction,
  type Rpc,
  type SolanaRpcApi,
} from "@solana/kit";
import { Keypair } from "@solana/web3.js";

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
  throw new Error(
    `Cannot normalise keypair: ${typeof keypairInput}, keys=${
      keypairInput ? Object.keys(keypairInput).join(",") : "null"
    }`,
  );
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
  keypairInput: any,
  cache?: Map<string, TransactionSigner>,
): Promise<TransactionSigner> {
  const bytes = toSecretKeyBytes(keypairInput);
  const signer = await createKeyPairSignerFromBytes(bytes);
  if (!cache) return signer;
  const existing = cache.get(signer.address);
  if (existing) return existing;
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
  if (v && typeof v === "object") {
    if (typeof v.S === "string") return address(v.S);
    if (typeof v.B3 === "string") return address(v.B3);
    if (typeof v.toBase58 === "function") return address(v.toBase58());
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

  const signedTx = await signTransactionMessageWithSigners(txMessage);
  const signature = getSignatureFromTransaction(signedTx);

  const sendAndConfirm = sendAndConfirmTransactionFactory({
    rpc,
    rpcSubscriptions,
  });
  await sendAndConfirm(signedTx, { commitment: "confirmed" });
  return signature;
}
