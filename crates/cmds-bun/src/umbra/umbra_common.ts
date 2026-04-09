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
import { PublicKey } from "@solana/web3.js";
import type { Context } from "@space-operator/flow-lib-bun";

export const INDEXER_ENDPOINT =
  "https://acqzie0a1h.execute-api.eu-central-1.amazonaws.com";
export const RELAYER_ENDPOINT =
  "https://6yn4ndrv2i.execute-api.eu-central-1.amazonaws.com";

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
    ? INDEXER_ENDPOINT
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
