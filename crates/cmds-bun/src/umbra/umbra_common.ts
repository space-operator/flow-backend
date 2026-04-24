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
  createSignerFromPrivateKeyBytes,
  getUmbraClient,
  getUmbraRelayer,
} from "@umbra-privacy/sdk";
import * as snarkjs from "snarkjs";
import { Keypair, PublicKey } from "@solana/web3.js";
import { BaseCommand, Value, type Context } from "@space-operator/flow-lib-bun";

export const INDEXER_ENDPOINT_MAINNET =
  "https://acqzie0a1h.execute-api.eu-central-1.amazonaws.com";
export const INDEXER_ENDPOINT_DEVNET =
  "https://utxo-indexer.api-devnet.umbraprivacy.com";
// Backward-compatible alias; prefer the network-specific constants above.
export const INDEXER_ENDPOINT = INDEXER_ENDPOINT_MAINNET;
export const RELAYER_ENDPOINT_MAINNET =
  "https://6yn4ndrv2i.execute-api.eu-central-1.amazonaws.com";
// Devnet relayer inferred from the indexer URL pattern
// (utxo-indexer.api-devnet.umbraprivacy.com → relayer.api-devnet.umbraprivacy.com).
// If Umbra publishes a different devnet relayer domain, override via env or update here.
export const RELAYER_ENDPOINT_DEVNET =
  "https://relayer.api-devnet.umbraprivacy.com";
// Backward-compatible alias; prefer the network-specific constants above.
export const RELAYER_ENDPOINT = RELAYER_ENDPOINT_MAINNET;

const SUPPORTED_NETWORKS = ["mainnet", "devnet"] as const;

function bigintSafeReplacer(_key: string, value: unknown): unknown {
  return typeof value === "bigint" ? value.toString() : value;
}

export function safeJsonStringify(value: unknown, spacing?: number): string {
  try {
    return JSON.stringify(value, bigintSafeReplacer, spacing);
  } catch {
    return String(value);
  }
}

type UmbraErrorDetails = {
  phase: string;
  name: string;
  message: string;
  cause?: string;
  stack?: string;
};

export function describeUmbraError(error: unknown): UmbraErrorDetails {
  const maybeError = error as
    | {
      phase?: unknown;
      name?: unknown;
      message?: unknown;
      cause?: unknown;
      stack?: unknown;
    }
    | null
    | undefined;

  const cause = maybeError?.cause;
  return {
    phase: typeof maybeError?.phase === "string"
      ? maybeError.phase
      : typeof maybeError?.name === "string"
      ? maybeError.name
      : "unknown",
    name: typeof maybeError?.name === "string" ? maybeError.name : "Error",
    message: typeof maybeError?.message === "string"
      ? maybeError.message
      : error instanceof Error
      ? error.message
      : String(error),
    cause: cause === undefined
      ? undefined
      : cause instanceof Error
      ? cause.message
      : typeof cause === "string"
      ? cause
      : safeJsonStringify(cause),
    stack: typeof maybeError?.stack === "string" ? maybeError.stack : undefined,
  };
}

export function logUmbraError(
  scope: string,
  error: unknown,
): UmbraErrorDetails {
  const details = describeUmbraError(error);
  console.error(`[${scope}] failed at phase: ${details.phase}`);
  console.error(`[${scope}] name: ${details.name}`);
  console.error(`[${scope}] message: ${details.message}`);
  if (details.cause) {
    console.error(`[${scope}] cause: ${details.cause}`);
  }
  if (details.stack) {
    console.error(`[${scope}] stack: ${details.stack}`);
  }
  return details;
}

export function wrapZkProver<
  T extends { prove: (inputs: any) => Promise<any> },
>(
  scope: string,
  prover: T,
): T {
  return {
    ...prover,
    async prove(inputs: any) {
      const startedAt = Date.now();
      console.log(`[${scope}] phase: zk_proof_generation_start`);
      try {
        const result = await prover.prove(inputs);
        console.log(
          `[${scope}] phase: zk_proof_generation_complete (${
            Date.now() - startedAt
          }ms)`,
        );
        return result;
      } catch (error) {
        logUmbraError(`${scope}:zk_prover`, error);
        throw error;
      }
    },
  };
}

export function getPrimarySignature(result: unknown): string {
  if (typeof result === "string") {
    return result;
  }

  if (Array.isArray(result)) {
    return result.map(String).join(",");
  }

  if (typeof result === "object" && result !== null) {
    const record = result as Record<string, unknown>;
    for (
      const key of [
        "signature",
        "createUtxoSignature",
        "callbackSignature",
        "createProofAccountSignature",
        "rentClaimSignature",
        "closeProofAccountSignature",
      ]
    ) {
      const value = record[key];
      if (typeof value === "string" && value.length > 0) {
        return value;
      }
    }
  }

  return safeJsonStringify(result);
}

/**
 * Resolve signer bytes for an Umbra node from its raw inputs.
 *
 * Accepts either:
 * - `inputs.pubkey` (base58 string) — signs via the flow framework's
 *   wallet adapter through ctx.requestSignature / ctx.requestMessageSignature.
 *   Preferred for user-initiated flows: the private key never leaves the
 *   wallet. The Umbra SDK's master seed is derived from signMessage, so
 *   this path supports ZK operations too.
 * - `inputs.keypair` — a `@solana/web3.js` Keypair object as delivered by
 *   the SO wallet adapter (any wallet node: hardcoded / generated /
 *   api_input). The framework materialises the Keypair for us; we pull
 *   its `.secretKey` (64-byte secret ∥ public). Intended for automation
 *   where a programmatic caller hands us the private half through the
 *   normal wallet binding machinery — no raw-bytes plumbing in the flow
 *   graph.
 *
 *   Also accepts raw bytes / ArrayLike<number> for backward compatibility
 *   with any legacy callers that wired bytes into this port before the
 *   type_bounds tightened to `keypair`.
 *
 * If both are provided, `keypair` wins (explicit-over-implicit).
 */
export function resolveUmbraSignerBytes(inputs: {
  keypair?: unknown;
  pubkey?: unknown;
}): Uint8Array {
  if (inputs.keypair !== undefined && inputs.keypair !== null) {
    const kp: any = inputs.keypair;
    // Already raw bytes wired in.
    if (kp instanceof Uint8Array) return kp;

    const secretKeyBytes = extractSecretKeyBytes(kp);
    if (secretKeyBytes) {
      return secretKeyBytes;
    }

    // Flow wallet nodes can emit adapter-wallet objects on a `keypair`
    // port when the backing wallet is user-controlled and the secret key
    // is not available to the backend.
    const adapterPubkeyBytes = extractAdapterWalletPubkeyBytes(kp);
    if (adapterPubkeyBytes) {
      return adapterPubkeyBytes;
    }

    // Legacy: caller wired in a plain array / ArrayLike<number>.
    if (typeof kp?.length === "number") {
      return new Uint8Array(kp as ArrayLike<number>);
    }

    throw new Error(
      "Umbra node received an unsupported `keypair` input shape. Expected a Solana Keypair, 64-byte secret key, or Flow adapter wallet object with `public_key`.",
    );
  }
  const pubkeyBytes = extractPubkeyBytes(inputs.pubkey);
  if (pubkeyBytes) {
    return pubkeyBytes;
  }
  throw new Error(
    "Umbra node requires either `pubkey` (base58 string or 32-byte array; signs via wallet adapter) or `keypair` (Solana Keypair from the framework wallet adapter; signs locally).",
  );
}

// Accept every shape a `pubkey`-typed or `string`-typed input may take in
// bun nodes: raw 32-byte array-likes (Uint8Array, Array, or index-keyed
// objects the way the flow runtime serializes pubkeys), IValue wrappers
// ({B3: "..."}, {S: "..."}), plain base58 strings, and PublicKey-likes.
function extractPubkeyBytes(value: unknown): Uint8Array | null {
  if (value === undefined || value === null) return null;
  if (value instanceof Uint8Array) {
    return value.length === 32 ? value : null;
  }
  if (Array.isArray(value) && value.length === 32) {
    return new Uint8Array(value as number[]);
  }
  if (typeof value === "string" && value.length > 0) {
    return new PublicKey(value).toBytes();
  }
  if (typeof value === "object") {
    const record = value as Record<string, unknown>;
    if (typeof record.B3 === "string" && record.B3.length > 0) {
      return new PublicKey(record.B3).toBytes();
    }
    if (typeof record.S === "string" && record.S.length > 0) {
      return new PublicKey(record.S).toBytes();
    }
    const maybePk = value as { toBase58?: () => string };
    if (typeof maybePk.toBase58 === "function") {
      return new PublicKey(maybePk.toBase58()).toBytes();
    }
    // Index-keyed plain object: {0: 255, 1: 130, ..., 31: 42}
    if ("0" in record && "31" in record) {
      const bytes = new Uint8Array(32);
      for (let i = 0; i < 32; i++) {
        bytes[i] = Number(record[String(i)] ?? 0);
      }
      return bytes;
    }
  }
  return null;
}

function extractSecretKeyBytes(value: unknown): Uint8Array | null {
  if (value === undefined || value === null) return null;
  if (value instanceof Uint8Array) {
    return value.length === 64 ? value : null;
  }
  if (Array.isArray(value) && value.length === 64) {
    return new Uint8Array(value as number[]);
  }
  if (typeof value === "object") {
    const record = value as Record<string, unknown>;
    const nestedSecretKey = record.secretKey;
    if (nestedSecretKey instanceof Uint8Array && nestedSecretKey.length === 64) {
      return nestedSecretKey;
    }
    if (Array.isArray(nestedSecretKey) && nestedSecretKey.length === 64) {
      return new Uint8Array(nestedSecretKey as number[]);
    }
    const directIndexed = extractIndexedBytes(record, 64);
    if (directIndexed) {
      return directIndexed;
    }
    if (typeof nestedSecretKey === "object" && nestedSecretKey !== null) {
      return extractIndexedBytes(nestedSecretKey as Record<string, unknown>, 64);
    }
  }
  return null;
}

function extractAdapterWalletPubkeyBytes(value: unknown): Uint8Array | null {
  if (!isRustWalletAdapterRecord(value)) return null;
  return extractPubkeyBytes(value.public_key);
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

function extractIndexedBytes(
  record: Record<string, unknown>,
  size: number,
): Uint8Array | null {
  if (!(String(0) in record) || !(String(size - 1) in record)) {
    return null;
  }
  const bytes = new Uint8Array(size);
  for (let i = 0; i < size; i++) {
    bytes[i] = Number(record[String(i)] ?? 0);
  }
  return bytes;
}

/**
 * Create an Umbra SDK client.
 *
 * Accepts either:
 * - 64-byte keypair: signs locally (works for all operations)
 * - 32-byte pubkey + ctx: signs via framework wallet adapter
 *   (works for transaction signing and message signing when the
 *    connected wallet client supports both)
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
  const indexerApiEndpoint = network === "mainnet"
    ? INDEXER_ENDPOINT_MAINNET
    : network === "devnet"
    ? INDEXER_ENDPOINT_DEVNET
    : undefined;

  const transactionForwarder = ctx ? createFrameworkForwarder(ctx) : undefined;

  return getUmbraClient(
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
 * signMessage is routed through ctx.requestMessageSignature(),
 * which allows adapter-based wallets to support anonymous
 * registration when the connected client exposes signMessage.
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

    async signMessage(message: Uint8Array) {
      const { signature } = await ctx.requestMessageSignature(pubkey, message);

      return {
        message,
        signature,
        signer: address,
      };
    },
  };
}

// ── Tests (only run under `bun test`, safe to import elsewhere) ───────
import { test, expect, describe } from "bun:test";
try {
  describe("resolveUmbraSignerBytes", () => {
    const probe = new BaseCommand({
      type: "bun",
      node_id: "umbra_signer_probe",
      config: {},
      outputs: [],
      inputs: [
        {
          id: "pubkey",
          name: "pubkey",
          type_bounds: ["pubkey", "string"],
          required: false,
          passthrough: false,
        },
        {
          id: "keypair",
          name: "keypair",
          type_bounds: ["keypair"],
          required: false,
          passthrough: false,
        },
      ],
    });

    test("uses adapter wallet object from Flow wallet output", () => {
      const pubkey = new PublicKey(new Uint8Array(32).fill(7));
      const result = resolveUmbraSignerBytes({
        keypair: {
          public_key: pubkey.toBytes(),
          token: null,
        },
      });

      expect(result.length).toBe(32);
      expect(Array.from(result)).toEqual(Array.from(pubkey.toBytes()));
    });

    test("rejects non-Rust wallet-shaped adapter objects on the keypair port", () => {
      const pubkey = new PublicKey(new Uint8Array(32).fill(11));
      expect(() =>
        resolveUmbraSignerBytes({
          keypair: {
            publicKey: pubkey.toBytes(),
          },
        })
      ).toThrow("unsupported `keypair` input shape");
    });

    test("uses local keypair secret key when available", () => {
      const secretKey = new Uint8Array(64).fill(9);
      const result = resolveUmbraSignerBytes({
        keypair: {
          secretKey,
        },
      });

      expect(result.length).toBe(64);
      expect(Array.from(result)).toEqual(Array.from(secretKey));
    });

    test("accepts Flow wallet pubkey output through Bun input deserialization", () => {
      const pubkey = Keypair.generate().publicKey;
      const inputs = probe.deserializeInputs({
        pubkey: Value.PublicKey(pubkey),
      });

      const result = resolveUmbraSignerBytes(inputs);
      expect(result.length).toBe(32);
      expect(new PublicKey(result).toBase58()).toBe(pubkey.toBase58());
    });

    test("accepts Flow wallet keypair output for local wallets through Bun input deserialization", () => {
      const keypair = Keypair.generate();
      const inputs = probe.deserializeInputs({
        keypair: Value.Keypair(keypair),
      });

      const result = resolveUmbraSignerBytes(inputs);
      expect(result.length).toBe(64);
      expect(new PublicKey(result.slice(32)).toBase58()).toBe(
        keypair.publicKey.toBase58(),
      );
    });

    test("accepts Flow wallet keypair output for adapter wallets through Bun input deserialization", () => {
      const pubkey = Keypair.generate().publicKey;
      const inputs = probe.deserializeInputs({
        keypair: Value.fromJSON({
          M: {
            public_key: { B3: pubkey.toBase58() },
            token: { N: 0 },
          },
        }),
      });

      const result = resolveUmbraSignerBytes(inputs);
      expect(result.length).toBe(32);
      expect(new PublicKey(result).toBase58()).toBe(pubkey.toBase58());
    });
  });
} catch (_) {
  // Not running under `bun test`
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

  // Send a signed tx without waiting for confirmation. Used by the SDK
  // for rent-reclaim after deposit/withdraw — we want the main result to
  // return as soon as the primary tx confirms, and let the rent-reclaim
  // follow-up land on its own. Without this method, the SDK logs
  // "config.transactionForwarder.fireAndForget is not a function" and
  // surfaces a non-fatal rentClaimError.
  const fireAndForget = async (transaction: any): Promise<string> => {
    const wireBytes = encodeWireTransaction(transaction);
    return await ctx.solana.sendRawTransaction(wireBytes, {
      skipPreflight: true,
    });
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
    fireAndForget,
  };
}

/**
 * Create the Umbra relayer client (needed for UTXO claims).
 * Picks the mainnet or devnet relayer URL per the `network` argument;
 * defaults to mainnet when unspecified for backward compatibility.
 */
export function createRelayer(network?: string) {
  const apiEndpoint = network === "devnet"
    ? RELAYER_ENDPOINT_DEVNET
    : RELAYER_ENDPOINT_MAINNET;
  return getUmbraRelayer({ apiEndpoint });
}

// ── Rust Native Prover ──────────────────────────────────────────────────

const CDN_BASE = "https://d3j9fjdkre529f.cloudfront.net";

/**
 * Resolve the path to the umbra-prover binary.
 * Checks $UMBRA_PROVER_BIN, then well-known build output paths.
 */
function resolveProverBin(): string {
  const envBin = process.env.UMBRA_PROVER_BIN;
  if (envBin) return envBin;

  const fs = require("node:fs") as typeof import("node:fs");

  // Try well-known paths relative to workspace root
  // The workspace node_modules symlink points to flow-backend/node_modules,
  // so we can derive the workspace root from there.
  const nodeModulesTarget = fs.realpathSync(
    require.resolve("@umbra-privacy/sdk/package.json"),
  );
  // nodeModulesTarget is like: /home/.../flow-backend/node_modules/@umbra-privacy/sdk/package.json
  const workspaceRoot = nodeModulesTarget.replace(/\/node_modules\/.*/, "");

  const candidates = [
    `${workspaceRoot}/target/debug/umbra-prover`,
    `${workspaceRoot}/target/release/umbra-prover`,
  ];

  for (const c of candidates) {
    try {
      if (fs.existsSync(c)) return c;
    } catch {
      // not found
    }
  }

  return "umbra-prover"; // fallback to PATH
}

/**
 * Download and cache a CDN asset to disk.
 */
async function ensureCached(url: string, cacheDir: string): Promise<string> {
  const fs = await import("node:fs");
  const path = await import("node:path");

  // Use URL path as cache key
  const relPath = url.replace(CDN_BASE + "/", "").replace(/\?.*/, "");
  const cachePath = path.join(cacheDir, relPath);

  if (fs.existsSync(cachePath)) {
    return cachePath;
  }

  console.log(`[rust-prover] downloading ${url}...`);
  const resp = await fetch(url);
  if (!resp.ok) throw new Error(`CDN ${resp.status} for ${url}`);

  const bytes = new Uint8Array(await resp.arrayBuffer());
  fs.mkdirSync(path.dirname(cachePath), { recursive: true });
  fs.writeFileSync(cachePath, bytes);
  console.log(`[rust-prover] cached ${cachePath} (${bytes.length} bytes)`);

  return cachePath;
}

/**
 * Fetch the CDN manifest and resolve asset URLs for a circuit.
 */
async function resolveCircuitAssets(
  circuitType: string,
  variant?: string,
  cacheDir?: string,
): Promise<{ wasmPath: string; zkeyPath: string }> {
  const cache = cacheDir ?? process.env.UMBRA_PROVER_CACHE_DIR ??
    `${process.env.HOME}/.cache/umbra-prover`;

  // Fetch manifest
  const manifestUrl = `${CDN_BASE}/manifest.json?t=${Date.now()}`;
  const manifestPath = `${cache}/manifest.json`;

  let manifest: any;
  const fs = await import("node:fs");
  try {
    const stat = fs.statSync(manifestPath);
    if (Date.now() - stat.mtimeMs < 3600_000) {
      manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
    }
  } catch { /* not cached or expired */ }

  if (!manifest) {
    const resp = await fetch(manifestUrl);
    const text = await resp.text();
    manifest = JSON.parse(text);
    fs.mkdirSync(cache, { recursive: true });
    fs.writeFileSync(manifestPath, text);
  }

  // Resolve URLs — assets are nested under manifest.assets
  const assets = manifest.assets ?? manifest;
  const entry = variant ? assets[circuitType]?.[variant] : assets[circuitType];
  if (!entry) {
    throw new Error(
      `Circuit ${circuitType}${variant ? `/${variant}` : ""} not in manifest`,
    );
  }

  const zkeyRel = entry.zkey ?? entry.url ?? entry;
  const wasmRel = (typeof zkeyRel === "string" ? zkeyRel : "").replace(
    ".zkey",
    ".wasm",
  );

  const zkeyUrl = zkeyRel.startsWith("http")
    ? zkeyRel
    : `${CDN_BASE}/${zkeyRel}`;
  const wasmUrl = wasmRel.startsWith("http")
    ? wasmRel
    : `${CDN_BASE}/${wasmRel}`;

  const zkeyPath = await ensureCached(zkeyUrl, cache);
  const wasmPath = await ensureCached(wasmUrl, cache);

  return { wasmPath, zkeyPath };
}

/**
 * Generate witness from circom WASM using snarkjs (single-threaded, no web workers).
 * Returns the witness as an array of decimal strings.
 */
async function generateWitness(
  wasmPath: string,
  circuitInputs: any,
): Promise<string[]> {
  const fs = await import("node:fs");
  const os = await import("node:os");
  const path = await import("node:path");

  console.log("[rust-prover] generating witness...");
  const startedAt = Date.now();

  // snarkjs.wtns.calculate writes to a file
  const wtnsFile = path.join(os.tmpdir(), `umbra-witness-${Date.now()}.wtns`);

  try {
    await snarkjs.wtns.calculate(circuitInputs, wasmPath, wtnsFile);

    // Read the witness file and export as JSON
    const witnessJson = await snarkjs.wtns.exportJson(wtnsFile);

    // witnessJson is an array of BigInts
    const witness: string[] = witnessJson.map((v: any) => v.toString());

    console.log(
      `[rust-prover] witness: ${witness.length} elements (${
        Date.now() - startedAt
      }ms)`,
    );
    return witness;
  } finally {
    // Clean up temp file
    try {
      fs.unlinkSync(wtnsFile);
    } catch { /* ignore */ }
  }
}

/**
 * Create a prover that uses Rust for Groth16 proof generation.
 *
 * The Bun side handles:
 * - CDN asset download + caching
 * - WASM witness generation (single-threaded snarkjs, no web workers)
 *
 * The Rust binary handles:
 * - Loading the zkey into arkworks ProvingKey
 * - Groth16 proof generation
 * - Proof serialization to Umbra byte layout
 */
export function createRustProver(circuitType: string, variant?: string) {
  const proverBin = resolveProverBin();

  return {
    // For claim provers that need maxUtxoCapacity
    maxUtxoCapacity: 1,

    async prove(circuitInputs: any, nLeaves?: number) {
      const actualVariant = variant ?? (nLeaves ? `n${nLeaves}` : undefined);

      // 1. Resolve CDN assets
      console.log(
        `[rust-prover] resolving assets for ${circuitType}${
          actualVariant ? `/${actualVariant}` : ""
        }...`,
      );
      const { wasmPath, zkeyPath } = await resolveCircuitAssets(
        circuitType,
        actualVariant,
      );

      // 2. Generate witness in Bun (single-threaded WASM, no web workers)
      const witness = await generateWitness(wasmPath, circuitInputs);

      // 3. Call Rust binary for Groth16 proving
      console.log(`[rust-prover] calling ${proverBin} --zkey ${zkeyPath}`);
      const startedAt = Date.now();

      const proc = Bun.spawn([proverBin, "--zkey", zkeyPath], {
        stdin: "pipe",
        stdout: "pipe",
        stderr: "pipe",
        cwd: "/tmp",
      });

      // Write witness as JSON array of decimal strings
      const witnessJson = JSON.stringify(witness);
      proc.stdin.write(witnessJson);
      proc.stdin.end();

      const [stdout, stderr] = await Promise.all([
        new Response(proc.stdout).text(),
        new Response(proc.stderr).text(),
      ]);
      const exitCode = await proc.exited;

      if (stderr) {
        // Log Rust prover progress (sent to stderr)
        for (const line of stderr.split("\n").filter(Boolean)) {
          console.log(`[rust-prover] ${line}`);
        }
      }

      if (exitCode !== 0) {
        throw new Error(`umbra-prover exited ${exitCode}: ${stderr}`);
      }

      // 4. Parse proof output
      const result = JSON.parse(stdout);

      const proofA = hexToBytes(result.proofA);
      const proofB = hexToBytes(result.proofB);
      const proofC = hexToBytes(result.proofC);

      console.log(
        `[rust-prover] proof generated (${
          Date.now() - startedAt
        }ms): A=${proofA.length}B B=${proofB.length}B C=${proofC.length}B`,
      );

      return { proofA, proofB, proofC };
    },
  };
}

function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}
