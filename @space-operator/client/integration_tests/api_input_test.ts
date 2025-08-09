import { Value } from "../src/deps.ts";
import * as client from "../src/mod.ts";
import * as dotenv from "jsr:@std/dotenv";
import { createClient } from "npm:@supabase/supabase-js@2";
import { assertEquals } from "jsr:@std/assert";
import { checkNoErrors } from "./utils.ts";

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

Deno.test("submit", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });

  const flowId = 3730;
  const ws = owner.ws();
  await ws.authenticate();
  console.log(ws.getIdentity());
  const { flow_run_id } = await owner.startFlow(flowId, {});
  ws.subscribeFlowRunEvents(
    async (ev) => {
      // console.log(ev);
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

Deno.test("cancel", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });

  const flowId = 3730;
  const ws = owner.ws();
  await ws.authenticate();
  const { flow_run_id } = await owner.startFlow(flowId, {});
  let setNodeError: (value: unknown) => void;
  const nodeError = new Promise((resolve) => {
    setNodeError = resolve;
  });
  ws.subscribeFlowRunEvents(
    async (ev) => {
      // console.log(ev);
      if (ev.event === "ApiInput") {
        const resp = await fetch(ev.data.url, {
          method: "DELETE",
        });
        await resp.text();
      } else if (ev.event === "NodeError") {
        setNodeError(ev.data.error);
      }
    },
    flow_run_id,
  );
  assertEquals(await nodeError, "canceled by user");
  await ws.close();
});

Deno.test("timeout", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });

  const flowId = 3730;
  const ws = owner.ws();
  await ws.authenticate();
  const { flow_run_id } = await owner.startFlow(flowId, {});
  let setNodeError: (value: unknown) => void;
  const nodeError = new Promise((resolve) => {
    setNodeError = resolve;
  });
  ws.subscribeFlowRunEvents(
    (ev) => {
      // console.log(ev);
      if (ev.event === "NodeError") {
        setNodeError(ev.data.error);
      }
    },
    flow_run_id,
  );
  assertEquals(await nodeError, "timeout");
  await ws.close();
});
