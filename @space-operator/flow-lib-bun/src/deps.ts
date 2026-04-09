/**
 * Dependencies for flow-lib-bun.
 * npm equivalents of the JSR packages used in flow-lib.
 */
import _bs58 from "bs58";
import * as stableBase64 from "@stablelib/base64";
import * as web3 from "@solana/web3.js";
import * as msgpack from "@msgpack/msgpack";

// Re-export matching the same interface as the Deno version
export const bs58 = {
  encodeBase58: (data: Uint8Array): string => _bs58.encode(data),
  decodeBase58: (str: string): Uint8Array => _bs58.decode(str),
};

export const base64 = {
  encodeBase64: (data: Uint8Array): string => stableBase64.encode(data),
  decodeBase64: (str: string): Uint8Array => stableBase64.decode(str),
};

export { web3, msgpack };
