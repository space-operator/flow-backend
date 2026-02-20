import { Value } from "../src/deps.ts";
import * as client from "../src/mod.ts";
import * as dotenv from "@std/dotenv";
import { createClient } from "@supabase/supabase-js";
import { assert, assertEquals } from "@std/assert";
import { checkNoErrors, getEnv, getUuidEnv } from "./utils.ts";

dotenv.loadSync({
  export: true,
});

const anonKey = getEnv("ANON_KEY");
const apiKey = getEnv("APIKEY");
const supabaseUrl = "http://localhost:8000";
const START_FLOW_ID = getUuidEnv("FLOW_TEST_START_FLOW_ID");
const DENO_FLOW_ID = getUuidEnv("FLOW_TEST_DENO_FLOW_ID");
const INTERFLOW_FLOW_ID = getUuidEnv("FLOW_TEST_INTERFLOW_FLOW_ID");
const INTERFLOW_INSTRUCTIONS_FLOW_ID = getUuidEnv(
  "FLOW_TEST_INTERFLOW_INSTRUCTIONS_FLOW_ID",
);
const CONSTS_FLOW_ID = getUuidEnv("FLOW_TEST_CONSTS_FLOW_ID");

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
