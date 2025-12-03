import { bs58, Value, web3 } from "../src/deps.ts";
import * as client from "../src/mod.ts";
import * as dotenv from "jsr:@std/dotenv";
import { createClient } from "npm:@supabase/supabase-js@2";
import { assert, assertEquals } from "jsr:@std/assert";
import { LAMPORTS_PER_SOL } from "npm:@solana/web3.js@^1.91.4";
import * as nacl from "npm:tweetnacl";
import { decodeBase64 } from "jsr:@std/encoding@0.221/base64";
import { checkNoErrors } from "./utils.ts";
import { encodeBase58 } from "jsr:@std/encoding@0.221/base58";

dotenv.loadSync({
  export: true,
});

function ed25519SignText(keypair: web3.Keypair, message: string): Uint8Array {
  return nacl.default.sign.detached(
    new TextEncoder().encode(message),
    keypair.secretKey,
  );
}

function getEnv(key: string): string {
  const env = Deno.env.get(key);
  if (env === undefined) throw new Error(`no env ${key}`);
  return env;
}

const anonKey = getEnv("ANON_KEY");
const apiKey = getEnv("APIKEY");
const supabaseUrl = "http://localhost:8000";

Deno.test("run x402", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });
  const ownerKeypair = web3.Keypair.fromSecretKey(
    bs58.decodeBase58(getEnv("KEYPAIR")),
  );

  const flowId = 3643;
  const id = await owner.deployFlow(flowId);

  const starterKeypair = web3.Keypair.generate();
  const starter = new client.Client({
    host: "http://localhost:8080",
    anonKey,
  });
  starter.setToken(starterKeypair.publicKey.toString());
  const { flow_run_id, token } = await starter.startDeployment(
    {
      id,
    },
    {
      inputs: new Value({
        n: 2,
      }).M!,
    },
  );
  starter.setToken(token);

  const result = await starter.getFlowOutput(flow_run_id);
  const { out, count } = result.toJSObject();
  assertEquals(count, 112);
  assertEquals(out, 1);

  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);
  await checkNoErrors(sup, flow_run_id);
});
