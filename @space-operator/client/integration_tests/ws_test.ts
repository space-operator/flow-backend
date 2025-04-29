import { Value } from "../src/deps.ts";
import * as client from "../src/mod.ts";
import * as dotenv from "jsr:@std/dotenv";
import { createClient, type SupabaseClient } from "npm:@supabase/supabase-js@2";
import { assertEquals } from "jsr:@std/assert";

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

async function checkNoErrors(
  sup: SupabaseClient<client.Database>,
  runId: client.FlowRunId,
) {
  const nodeErrors = await sup
    .from("node_run")
    .select("errors")
    .eq("flow_run_id", runId)
    .not("errors", "is", "null");
  if (nodeErrors.error) throw new Error(JSON.stringify(nodeErrors.error));
  const flowErrors = await sup
    .from("flow_run")
    .select("errors")
    .eq("id", runId)
    .not("errors", "is", "null");
  if (flowErrors.error) throw new Error(JSON.stringify(flowErrors.error));
  const errors = [
    ...flowErrors.data.flatMap((row) => row.errors),
    ...nodeErrors.data.flatMap((row) => row.errors),
  ];
  if (errors.length > 0) throw new Error(JSON.stringify(errors));
}

Deno.test("start flow", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });

  const flowId = 3730;
  const ws = owner.ws();
  await ws.authenticate();
  console.log(ws.getIdentity());
  const { flow_run_id } = await owner.startFlow(flowId, {
    inputs: new Value({
      a: 1,
      b: 2,
    }).M!,
  });
  ws.subscribeFlowRunEvents(
    async (ev) => {
      console.log(ev);
      if (ev.event === "ApiInput") {
        const resp = await fetch(ev.data.url, {
          method: "POST",
          headers: [["content-type", "application/json"]],
          body: JSON.stringify({ value: new Value("hello") }),
        });
        await resp.text();
      }
    },
    flow_run_id,
  );

  const result = await owner.getFlowOutput(flow_run_id);
  const c = result.toJSObject().c;
  assertEquals(c, "hello");

  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);
  await checkNoErrors(sup, flow_run_id);
  await ws.close();
});
