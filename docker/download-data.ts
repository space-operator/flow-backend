#!/usr/bin/env -S deno run --allow-env --allow-net --allow-read=.env,.env.defaults,.env.example --allow-write=export.json

import { load } from "@std/dotenv";
import { parseArgs } from "jsr:@std/cli/parse-args";

const args = parseArgs(Deno.args, {
  string: ["from"],
});
const from = args.from ?? "https://dev-api.spaceoperator.com";

function getEnv(key: string): string {
  const value = Deno.env.get(key);
  if (value === undefined) {
    throw new Error(`environment variable ${key} not found`);
  }
  return value;
}

async function main() {
  await load({ export: true });
  const APIKEY = getEnv("APIKEY");
  console.log(`Exporting data from ${from}`);
  const exportResp = await fetch(`${from}/data/export`, {
    method: "POST",
    headers: {
      "accept-encoding": "br, gzip",
      "x-api-key": APIKEY,
    },
  });
  if (exportResp.status !== 200) {
    console.error(await exportResp.text());
    Deno.exit(1);
  }
  const data = await exportResp.json();
  console.log("Saving to export.json");
  Deno.writeTextFile("export.json", JSON.stringify(data));
}

main();
