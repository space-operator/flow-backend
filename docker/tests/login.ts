import * as sol from "@solana/web3.js";
import { decodeBase58, encodeBase58 } from "@std/encoding";
import { load } from "@std/dotenv";
import { default as nacl } from "tweetnacl";

function getEnv(key: string): string {
  const value = Deno.env.get(key);
  if (value === undefined)
    throw new Error(`environment variable ${key} not found`);
  return value;
}

await load({ export: true });

const SERVER = `http://localhost:${getEnv("KONG_HTTP_PORT")}/flow-server`;

const keyB58 = Deno.env.get("KEYPAIR");
let key;
if (keyB58 !== undefined) {
  key = sol.Keypair.fromSecretKey(decodeBase58(keyB58));
} else {
  console.log("Generating random keypair");
  key = sol.Keypair.generate();
  console.log("key:", encodeBase58(key.secretKey));
}

const msg: string = await fetch(`${SERVER}/auth/init`, {
  method: "POST",
  body: JSON.stringify({ pubkey: key.publicKey.toBase58() }),
  headers: {
    "content-type": "application/json",
    apikey: getEnv("ANON_KEY"),
  },
})
  .then((resp) => resp.json())
  .then((json) => String(json.msg));

console.log("message to sign:");
console.log(msg);

const signature = nacl.sign.detached(
  new TextEncoder().encode(msg),
  key.secretKey
);

const signatureB58 = encodeBase58(signature);

const token = `${msg}.${signatureB58}`;

const authResult: string = await fetch(`${SERVER}/auth/confirm`, {
  method: "POST",
  body: JSON.stringify({ token }),
  headers: {
    "content-type": "application/json",
    apikey: getEnv("ANON_KEY"),
  },
}).then((resp) => resp.json());

console.log(authResult);
