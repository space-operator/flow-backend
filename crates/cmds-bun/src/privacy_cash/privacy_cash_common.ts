/**
 * Shared helpers for Privacy Cash bun nodes.
 *
 * Creates PrivacyCash SDK instances from signer input.
 * The SDK handles ZK proof generation (snarkjs), transaction building,
 * and relay submission internally.
 *
 * Privacy Cash requires a local secret key because it derives an encryption key
 * from the wallet. Flow adapter wallets only expose a public key + signing
 * service token, which is not enough for this SDK.
 */
import { PrivacyCash } from "privacycash";
import { Keypair, PublicKey } from "@solana/web3.js";

/** Devnet program ID for Privacy Cash */
export const DEVNET_PROGRAM_ID = "ATZj4jZ4FFzkvAcvk27DW9GRkgSbFnHo49fKKPQXU7VS";
/** Mainnet program ID for Privacy Cash */
export const MAINNET_PROGRAM_ID = "9fhQBbumKEFuXtMBDw8AaQyAjCorLGJQiS3skWZdQyQD";

export function resolvePrivacyCashKeypair(keypairInput: any): Keypair {
  if (keypairInput instanceof Keypair) {
    return keypairInput;
  }

  const secretKeyBytes = extractSecretKeyBytes(keypairInput);
  if (secretKeyBytes) {
    return Keypair.fromSecretKey(secretKeyBytes);
  }

  if (extractAdapterWalletPubkey(keypairInput)) {
    throw new Error(
      "Privacy Cash bun nodes require a local Solana keypair with secret key bytes. Flow adapter wallets are not supported because the Privacy Cash SDK derives an encryption key from the private key.",
    );
  }

  throw new Error(
    `Cannot create keypair from input: ${typeof keypairInput}, ` +
    `keys: ${keypairInput ? Object.keys(keypairInput).join(",") : "null"}`,
  );
}

/**
 * Create a PrivacyCash SDK client from a keypair input.
 */
export function createPrivacyCashClient(
  keypairInput: any,
  rpcUrl: string,
): PrivacyCash {
  const keypair = resolvePrivacyCashKeypair(keypairInput);

  // Pass secret key as number[] to avoid Keypair instanceof mismatch
  // between the framework's web3.js and the SDK's bundled web3.js
  return new PrivacyCash({
    RPC_url: rpcUrl,
    owner: Array.from(keypair.secretKey),
  });
}

export function toRecipientAddress(value: unknown): string {
  if (typeof value === "string" && value.length > 0) {
    return value;
  }
  if (value instanceof PublicKey) {
    return value.toBase58();
  }
  if (value instanceof Uint8Array && value.length === 32) {
    return new PublicKey(value).toBase58();
  }
  if (Array.isArray(value) && value.length === 32) {
    return new PublicKey(value).toBase58();
  }
  if (typeof value === "object" && value !== null) {
    const record = value as Record<string, unknown>;
    if (typeof record.S === "string" && record.S.length > 0) {
      return record.S;
    }
    if (typeof record.B3 === "string" && record.B3.length > 0) {
      return record.B3;
    }
    if (typeof (value as { toBase58?: () => string }).toBase58 === "function") {
      return (value as { toBase58: () => string }).toBase58();
    }
    const indexed = extractIndexedBytes(record, 32);
    if (indexed) {
      return new PublicKey(indexed).toBase58();
    }
  }
  throw new Error("Missing required input: recipient (Base58 public key)");
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
    const nestedSecretKey = record.secretKey ?? record._keypair;
    if (nestedSecretKey && typeof nestedSecretKey === "object") {
      const maybeSecret = (nestedSecretKey as Record<string, unknown>).secretKey ??
        nestedSecretKey;
      if (maybeSecret instanceof Uint8Array && maybeSecret.length === 64) {
        return maybeSecret;
      }
      if (Array.isArray(maybeSecret) && maybeSecret.length === 64) {
        return new Uint8Array(maybeSecret as number[]);
      }
      if (typeof maybeSecret === "object" && maybeSecret !== null) {
        return extractIndexedBytes(maybeSecret as Record<string, unknown>, 64);
      }
    }
    return extractIndexedBytes(record, 64);
  }
  return null;
}

function extractAdapterWalletPubkey(value: unknown): PublicKey | null {
  if (!isRustWalletAdapterRecord(value)) return null;
  try {
    return new PublicKey(toRecipientAddress(value.public_key));
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

import { describe, expect, test } from "bun:test";
try {
  describe("privacy_cash_common", () => {
    test("resolvePrivacyCashKeypair: accepts local keypair-like input", () => {
      const keypair = Keypair.generate();
      const resolved = resolvePrivacyCashKeypair({
        secretKey: Array.from(keypair.secretKey),
      });
      expect(Array.from(resolved.secretKey)).toEqual(Array.from(keypair.secretKey));
    });

    test("resolvePrivacyCashKeypair: rejects adapter wallets with a clear error", () => {
      const keypair = Keypair.generate();
      expect(() =>
        resolvePrivacyCashKeypair({
          public_key: keypair.publicKey.toBytes(),
          token: null,
        })
      ).toThrow("Flow adapter wallets are not supported");
    });

    test("resolvePrivacyCashKeypair: ignores non-Rust wallet-shaped plain objects", () => {
      const keypair = Keypair.generate();
      expect(() =>
        resolvePrivacyCashKeypair({
          publicKey: keypair.publicKey.toBytes(),
        })
      ).toThrow("Cannot create keypair from input");
    });

    test("toRecipientAddress: accepts PublicKey and pubkey wrappers", () => {
      const publicKey = Keypair.generate().publicKey;
      expect(toRecipientAddress(publicKey)).toBe(publicKey.toBase58());
      expect(toRecipientAddress({ B3: publicKey.toBase58() })).toBe(
        publicKey.toBase58(),
      );
    });
  });
} catch (_) {
  // Not running under `bun test`
}
