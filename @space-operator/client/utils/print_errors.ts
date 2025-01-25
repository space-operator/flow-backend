import { createClient } from "npm:@supabase/supabase-js@2";
import { Client, type Database } from "../src/mod.ts";

function getEnv(key: string): string {
  const env = Deno.env.get(key);
  if (env === undefined) throw new Error(`no env ${key}`);
  return env;
}
const anonKey = getEnv("ANON_KEY");
const c = new Client({
  host: "http://localhost:8080",
  supabaseUrl: "http://localhost:8000",
  anonKey,
  token: getEnv("APIKEY"),
});
const jwt = await c.claimToken();
const sup = createClient<Database>("http://localhost:8000", anonKey, {
  auth: {
    autoRefreshToken: false,
  },
});
await sup.auth.setSession(jwt);

const nodeErrors = await sup
  .from("node_run")
  .select("errors")
  .not("errors", "is", "null");
if (nodeErrors.error) throw new Error(JSON.stringify(nodeErrors.error));
const flowErrors = await sup
  .from("flow_run")
  .select("errors")
  .not("errors", "is", "null");
if (flowErrors.error) throw new Error(JSON.stringify(flowErrors.error));

const errors = [
  ...flowErrors.data.flatMap((row) => row.errors),
  ...nodeErrors.data.flatMap((row) => row.errors),
];
errors.forEach((error) => console.log(error));
