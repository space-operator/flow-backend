import { bs58, Value, web3 } from "../src/deps.ts";
import * as client from "../src/mod.ts";
import * as dotenv from "@std/dotenv";
import { createClient } from "@supabase/supabase-js";
import { assert, assertEquals } from "@std/assert";
import { checkNoErrors, getEnv } from "./utils.ts";

dotenv.loadSync({
  export: true,
});

const anonKey = getEnv("ANON_KEY");
const apiKey = getEnv("APIKEY");
const supabaseUrl = "http://localhost:8000";
const START_FLOW_ID = "6c949718-69e2-47c1-8b93-d56b8e34ec51"; // Add
const DENO_FLOW_ID = "c349c074-0f4f-41bd-976d-d8df32ba867a"; // Deno Add
const INTERFLOW_FLOW_ID = "b3c95f36-2a1c-4e33-be2a-28758a0c4b9d"; // Collatz
const INTERFLOW_INSTRUCTIONS_FLOW_ID =
  "69401e5a-375e-49d0-bb95-33c9d70eb582"; // Interflow Instructions
const CONSTS_FLOW_ID = "27b35933-7165-4da5-a2ea-a6342bbb3da7"; // Consts
const TRANSFER_SOL_FLOW_ID = "92b480ad-1a18-4a52-a459-4d5420890272"; // Transfer SOL

Deno.test("start flow", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });

  const { flow_run_id } = await owner.startFlow(START_FLOW_ID, {
    inputs: new Value({
      a: 1,
      b: 2,
    }).M!,
  });

  const result = await owner.getFlowOutput(flow_run_id);
  const c = result.toJSObject().c;
  assertEquals(c, 3);

  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);
  await checkNoErrors(sup, flow_run_id);
});

Deno.test("test deno node", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });

  const { flow_run_id } = await owner.startFlow(DENO_FLOW_ID, {});

  const result = await owner.getFlowOutput(flow_run_id);
  const c = result.toJSObject().pi;
  assertEquals(c, 3.14);

  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);
  await checkNoErrors(sup, flow_run_id);
});

Deno.test("interflow", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });

  const { flow_run_id } = await owner.startFlow(INTERFLOW_FLOW_ID, {
    inputs: new Value({
      n: 54,
    }).M!,
  });

  const result = await owner.getFlowOutput(flow_run_id);
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

Deno.test("interflow_instructions", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });

  const { flow_run_id } = await owner.startFlow(
    INTERFLOW_INSTRUCTIONS_FLOW_ID,
    {},
  );

  const result = await owner.getFlowOutput(flow_run_id);
  const { ins } = result.toJSObject();
  console.log(ins);
  assert(ins != null);

  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);
  await checkNoErrors(sup, flow_run_id); // there are node errors
});

Deno.test("consts", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });

  const { flow_run_id } = await owner.startFlow(CONSTS_FLOW_ID, {});

  await owner.getFlowOutput(flow_run_id);

  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);
  await checkNoErrors(sup, flow_run_id);
});

Deno.test({
  name: "start flow with direct keypair input over bearer auth",
  sanitizeOps: false,
  sanitizeResources: false,
  fn: async () => {
    const owner = new client.Client({
      host: "http://localhost:8080",
      anonKey,
      token: apiKey,
    });
    const jwt = await owner.claimToken();
    const bearer = new client.Client({
      host: "http://localhost:8080",
      anonKey,
      token: jwt.access_token,
    });
    const ownerKeypair = web3.Keypair.fromSecretKey(
      bs58.decodeBase58(getEnv("KEYPAIR")),
    );

    const { flow_run_id } = await bearer.startFlow(TRANSFER_SOL_FLOW_ID, {
      inputs: new Value({
        sender: Value.Keypair(ownerKeypair.secretKey),
        recipient: ownerKeypair.publicKey,
        amount: 0.000001,
      }).M!,
    });

    const result = await bearer.getFlowOutput(flow_run_id);
    assert(result.M?.signature != null);

    const sup = createClient<client.Database>(supabaseUrl, anonKey, {
      auth: { autoRefreshToken: false },
    });
    await sup.auth.setSession(jwt);
    await checkNoErrors(sup, flow_run_id);
  },
});
