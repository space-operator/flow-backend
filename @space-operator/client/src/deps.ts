export * as web3 from "npm:@solana/web3.js@1.94.0";
export * as lib from "jsr:@space-operator/flow-lib@0.10.0";
export * as bs58 from "jsr:@std/encoding@^0.221.0/base58";
export * as base64 from "jsr:@std/encoding@^0.221.0/base64";
export { Value, type IValue } from "jsr:@space-operator/flow-lib@0.10.0/value";
export { Buffer } from "node:buffer";
export type { Session as SupabaseSession } from "npm:@supabase/auth-js@2.64.4";

import * as nacl from "npm:tweetnacl@1.0.3";
export function naclSign(msg: Uint8Array, secretKey: Uint8Array): Uint8Array {
  return nacl.default.sign.detached(msg, secretKey);
}
