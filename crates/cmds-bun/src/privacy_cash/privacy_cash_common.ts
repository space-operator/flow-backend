/**
 * Shared helpers for Privacy Cash bun nodes.
 *
 * Creates PrivacyCash SDK instances from keypair input.
 * The SDK handles ZK proof generation (snarkjs), transaction building,
 * and relay submission internally.
 *
 * Accepts either:
 * - 64-byte Uint8Array (raw keypair bytes)
 * - web3.js Keypair object (from BaseCommand keypair deserialization)
 */
import { PrivacyCash } from "privacycash";
import { Keypair } from "@solana/web3.js";

/** Devnet program ID for Privacy Cash */
export const DEVNET_PROGRAM_ID = "ATZj4jZ4FFzkvAcvk27DW9GRkgSbFnHo49fKKPQXU7VS";
/** Mainnet program ID for Privacy Cash */
export const MAINNET_PROGRAM_ID = "9fhQBbumKEFuXtMBDw8AaQyAjCorLGJQiS3skWZdQyQD";

/**
 * Create a PrivacyCash SDK client from a keypair input.
 *
 * Handles multiple input formats:
 * - Uint8Array (64 bytes): raw keypair bytes
 * - Keypair object: web3.js Keypair from BaseCommand deserialization
 * - Any object with secretKey: duck-typed Keypair
 */
export function createPrivacyCashClient(
  keypairInput: any,
  rpcUrl: string,
): PrivacyCash {
  let keypair: Keypair;

  if (keypairInput instanceof Keypair) {
    keypair = keypairInput;
  } else if (keypairInput instanceof Uint8Array && keypairInput.length === 64) {
    keypair = Keypair.fromSecretKey(keypairInput);
  } else if (keypairInput?.secretKey) {
    // web3.js Keypair object (may be from different module instance)
    keypair = Keypair.fromSecretKey(new Uint8Array(keypairInput.secretKey));
  } else if (keypairInput?._keypair?.secretKey) {
    // Internal Keypair format
    keypair = Keypair.fromSecretKey(new Uint8Array(keypairInput._keypair.secretKey));
  } else {
    throw new Error(
      `Cannot create keypair from input: ${typeof keypairInput}, ` +
      `keys: ${keypairInput ? Object.keys(keypairInput).join(",") : "null"}`,
    );
  }

  // Pass secret key as number[] to avoid Keypair instanceof mismatch
  // between the framework's web3.js and the SDK's bundled web3.js
  return new PrivacyCash({
    RPC_url: rpcUrl,
    owner: Array.from(keypair.secretKey),
  });
}
