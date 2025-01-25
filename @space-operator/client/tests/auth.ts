import * as client from "../src/mod.ts";
import { web3, bs58 } from "../src/deps.ts";
import * as dotenv from "jsr:@std/dotenv";
import * as nacl from "npm:tweetnacl";

function ed25519SignText(keypair: web3.Keypair, message: string): Uint8Array {
  return nacl.default.sign.detached(
    new TextEncoder().encode(message),
    keypair.secretKey
  );
}

dotenv.loadSync({
  export: true,
});

const anonKey = Deno.env.get("ANON_KEY");
if (!anonKey) throw new Error("no ANON_KEY");

const c = new client.Client({
  host: "http://localhost:8080",
  supabaseUrl: "http://localhost:8000",
  anonKey,
});

function keypair_from_env() {
  const value = Deno.env.get("TEST_KEYPAIR");
  if (!value) return undefined;
  return web3.Keypair.fromSecretKey(bs58.decodeBase58(value));
}

const keypair = keypair_from_env() ?? web3.Keypair.generate();
console.log("using", keypair.publicKey.toBase58());

const run = async (keypair: web3.Keypair) => {
  const msg = await c.initAuth(keypair.publicKey);
  const sig = ed25519SignText(keypair, msg);
  const result = await c.confirmAuth(msg, sig);
  const sup = await c.supabase(result.session);
  return await sup.auth.getUser();
};

const results = await Promise.all([
  run(keypair),
  run(keypair),
  run(keypair),
  run(web3.Keypair.generate()),
  run(web3.Keypair.generate()),
  run(web3.Keypair.generate()),
]);

console.log(results);
