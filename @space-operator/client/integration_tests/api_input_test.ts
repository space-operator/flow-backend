import { Value } from "../src/deps.ts";
import * as client from "../src/mod.ts";
import * as dotenv from "@std/dotenv";
import { createClient } from "@supabase/supabase-js";
import { assertEquals } from "@std/assert";
import { checkNoErrors, getEnv } from "./utils.ts";
// import { Application, Router } from "@oak/oak";

dotenv.loadSync({
  export: true,
});

const anonKey = getEnv("ANON_KEY");
const apiKey = getEnv("APIKEY");
const supabaseUrl = "http://localhost:8000";
const API_INPUT_FLOW_ID = "78a7e826-7697-48cb-a2c0-67ad1be4e970"; // API Input

function fixUrl(url: string) {
  return url.replace("flow-server", "localhost");
}

Deno.test("submit", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });

  const ws = owner.ws();
  await ws.authenticate();
  console.log(ws.getIdentity());
  const { flow_run_id } = await owner.startFlow(API_INPUT_FLOW_ID, {});
  ws.subscribeFlowRunEvents(
    async (ev) => {
      // console.log(ev);
      if (ev.event === "ApiInput") {
        const resp = await fetch(fixUrl(ev.data.url), {
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

  const ws = owner.ws();
  await ws.authenticate();
  const { flow_run_id } = await owner.startFlow(API_INPUT_FLOW_ID, {});
  let setNodeError: (value: unknown) => void;
  const nodeError = new Promise((resolve) => {
    setNodeError = resolve;
  });
  ws.subscribeFlowRunEvents(
    async (ev) => {
      // console.log(ev);
      if (ev.event === "ApiInput") {
        const resp = await fetch(fixUrl(ev.data.url), {
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

  const ws = owner.ws();
  await ws.authenticate();
  const { flow_run_id } = await owner.startFlow(API_INPUT_FLOW_ID, {});
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

/*
const router = new Router();
router.post("/webhook", async (ctx) => {
  const info = await ctx.request.body.json();
  const url = info.url!;
  console.log(info);
  const resp = await fetch(url, {
    method: "POST",
    headers: [["content-type", "application/json"]],
    body: JSON.stringify({ value: new Value("hello") }),
  });
  await resp.text();
  ctx.response.body = "ok";
});
const app = new Application();
let setPort = (_x: unknown) => {};
const portPromise = new Promise((resolve) => {
  setPort = resolve;
});
app.addEventListener("listen", (ev) => {
  setPort(ev.port);
});
app.use(router.routes());
app.use(router.allowedMethods());
app.listen({
  port: 0,
});
*/

Deno.test("webhook", async () => {
  // const port = await portPromise as number;
  // console.log("listening on port ", port);

  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });
  const ws = owner.ws();
  await ws.authenticate();

  const { flow_run_id } = await owner.startFlow(API_INPUT_FLOW_ID, {
    inputs: new Value({
      "webhook_url": `http://webhook/webhook`,
    }).M!,
  });
  ws.subscribeFlowRunEvents(
    async (ev) => {
      if (ev.event === "FlowLog") {
        console.log(ev.data.content);
      }
      if (ev.event === "NodeLog") {
        console.log(ev.data.content);
      }
    },
    flow_run_id,
  );

  const result = await owner.getFlowOutput(flow_run_id);
  ws.close();
  const c = result.toJSObject().c;
  assertEquals(c, "hello");

  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);
  await checkNoErrors(sup, flow_run_id);
});
