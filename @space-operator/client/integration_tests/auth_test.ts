import * as client from "../src/mod.ts";
import { web3, bs58 } from "../src/deps.ts";
import * as dotenv from "@std/dotenv";
import * as nacl from "tweetnacl";
import { createClient } from "@supabase/supabase-js";
import type { UserResponse } from "@supabase/auth-js";
import { assert } from "@std/assert";
import { getEnv } from "./utils.ts";

function ed25519SignText(keypair: web3.Keypair, message: string): Uint8Array {
  return nacl.default.sign.detached(
    new TextEncoder().encode(message),
    keypair.secretKey
  );
}

dotenv.loadSync({
  export: true,
});

const anonKey = getEnv("ANON_KEY");

const supabaseUrl = "http://localhost:8000";

function keypair_from_env(): web3.Keypair {
  const value = getEnv("KEYPAIR");
  return web3.Keypair.fromSecretKey(bs58.decodeBase58(value));
}

const run = async (keypair: web3.Keypair): Promise<UserResponse> => {
  const c = new client.Client({
    host: "http://localhost:8080",
    anonKey,
  });
  const msg = await c.initAuth(keypair.publicKey);
  const sig = ed25519SignText(keypair, msg);
  const result = await c.confirmAuth(msg, sig);
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: {
      autoRefreshToken: false,
    },
  });
  await sup.auth.setSession(result.session);
  return await sup.auth.getUser();
};

Deno.test("test existing user", async () => {
  const keypair = keypair_from_env();
  const user = await run(keypair);
  assert(user.error == null);
});

Deno.test("test new user", async () => {
  const keypair = web3.Keypair.generate();
  const user = await run(keypair);
  assert(user.error == null);
});
