import { load } from "@std/dotenv";

function nonNull<T>(v: T | undefined | null): T {
  if (v === null || v === undefined) throw "value is null";
  return v;
}

await load({ export: true });
const SERVICE_ROLE_KEY = nonNull(Deno.env.get("SERVICE_ROLE_KEY"));
const APIKEY = nonNull(Deno.env.get("APIKEY"));
console.log("exporting data from https://dev-api.spaceoperator.com");
const resp = await fetch("https://dev-api.spaceoperator.com/data/export", {
  method: "POST",
  headers: {
    "accept-encoding": "br, gzip",
    "x-api-key": APIKEY,
  },
});
if (resp.status !== 200) {
  console.log(await resp.text());
  Deno.exit(1);
}
const data = await resp.json();
Deno.exit(0);
console.log("importing data to http://localhost:8080");
await fetch("http://localhost:8080/data/import", {
  headers: {
    authorization: `BEARER ${SERVICE_ROLE_KEY}`,
    "content-type": "application/json",
  },
  body: JSON.stringify(data),
  method: "POST",
});
