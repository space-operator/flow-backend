import { Value } from "../src/deps.ts";
import * as client from "../src/mod.ts";
import * as dotenv from "jsr:@std/dotenv";
import { createClient } from "npm:@supabase/supabase-js@2";
import { assertEquals } from "jsr:@std/assert";
import { checkNoErrors } from "./utils";

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

Deno.test("start flow", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });

  const flowId = 3675;
  const { flow_run_id } = await owner.startFlow(flowId, {
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

Deno.test("interflow", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });

  const flowId = 3623;
  const { flow_run_id } = await owner.startFlow(flowId, {
    inputs: new Value({
      n: 12345,
    }).M!,
  });

  const result = await owner.getFlowOutput(flow_run_id);
  const { out, count } = result.toJSObject();
  assertEquals(count, 50);
  assertEquals(out, 1);

  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);
  await checkNoErrors(sup, flow_run_id);
});
