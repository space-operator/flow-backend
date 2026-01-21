import { bs58, Value, web3 } from "../src/deps.ts";
import * as client from "../src/mod.ts";
import * as dotenv from "@std/dotenv";
import { createClient } from "@supabase/supabase-js";
import { assertEquals } from "@std/assert";
import { createKeyPairSignerFromBytes } from "@solana/kit";
import { checkNoErrors } from "./utils.ts";
import { wrapFetchWithPayment, x402Client } from "@x402/fetch";
import { registerExactSvmScheme } from "@x402/svm/exact/client";

dotenv.loadSync({
  export: true,
});

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
  const usdcKeypair = await createKeyPairSignerFromBytes(
    bs58.decodeBase58(getEnv("USDC_KEYPAIR")),
  );
  const x402 = new x402Client();
  registerExactSvmScheme(x402, { signer: usdcKeypair });
  const xFetch = wrapFetchWithPayment(fetch, x402) as typeof globalThis.fetch;

  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);

  const flowId = 3623;
  const id = await owner.deployFlow(flowId);

  await sup
    .from("flow_deployments")
    .update({ start_permission: "Anonymous" })
    .eq("id", id);

  const walletId = await sup
    .from("wallets")
    .select("id")
    .eq(
      "name",
      "Main wallet",
    )
    .single()
    .then((result) => {
      return result.data?.id;
    });

  if (walletId == null) {
    throw "could not find wallet";
  }

  const user_id = (await sup.auth.getUser()).data.user?.id;

  await sup.from("flow_deployments_x402_fees").insert(
    {
      user_id: user_id!,
      deployment_id: id,
      amount: 0.01,
      enabled: true,
      network: "solana-devnet",
      pay_to: walletId,
    },
  );

  const starterKeypair = web3.Keypair.generate();
  const starter = new client.Client({
    host: "http://localhost:8080",
    anonKey,
  });
  starter.setFetch(xFetch);
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
  assertEquals(count, 1);
  assertEquals(out, 1);

  await checkNoErrors(sup, flow_run_id);

  await sup.from("flow_deployments").delete().eq("id", id);
});
