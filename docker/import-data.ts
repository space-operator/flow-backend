#!/usr/bin/env -S deno run --allow-env --allow-net --allow-read

import { load } from "@std/dotenv";

function getEnv(key: string): string {
  const value = Deno.env.get(key);
  if (value === undefined)
    throw new Error(`environment variable ${key} not found`);
  return value;
}

await load({ export: true });
const SERVICE_ROLE_KEY = getEnv("SERVICE_ROLE_KEY");
const APIKEY = getEnv("APIKEY");
console.log("Exporting data from https://dev-api.spaceoperator.com");
const exportResp = await fetch(
  "https://dev-api.spaceoperator.com/data/export",
  {
    method: "POST",
    headers: {
      "accept-encoding": "br, gzip",
      "x-api-key": APIKEY,
    },
  }
);
if (exportResp.status !== 200) {
  console.error(await exportResp.text());
  Deno.exit(1);
}
const data = await exportResp.json();
const SERVER = `http://localhost:${getEnv("KONG_HTTP_PORT")}/flow-server`;
console.log(`Importing data to ${SERVER}`);
const importResp = await fetch(
  `${SERVER}/data/import`,
  {
    headers: {
      authorization: `Bearer ${SERVICE_ROLE_KEY}`,
      "content-type": "application/json",
    },
    body: JSON.stringify(data),
    method: "POST",
  }
);
if (importResp.status !== 200) {
  console.error("Error:", importResp.status);
  console.error(await importResp.text());
  Deno.exit(1);
}
