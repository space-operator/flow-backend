import * as client from "../src/mod.ts";
import * as dotenv from "jsr:@std/dotenv";

dotenv.loadSync({
  export: true,
});

function getEnv(key: string): string {
  const env = Deno.env.get(key);
  if (env === undefined) throw new Error(`no env ${key}`);
  return env;
}

const anonKey = getEnv("ANON_KEY");

Deno.test("test export", async () => {
  const c = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: getEnv("APIKEY"),
  });

  await c.export();
});
