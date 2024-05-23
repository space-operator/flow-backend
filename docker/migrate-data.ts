import { load } from "@std/dotenv";

function nonNull<T>(v: T | undefined | null): T {
  if (v === null || v === undefined) throw "value is null";
  return v;
}

await load({ export: true });
const SERVICE_ROLE_KEY = nonNull(Deno.env.get("SERVICE_ROLE_KEY"));
const APIKEY = nonNull(Deno.env.get("APIKEY"));
console.log("exporting data from https://dev-api.spaceoperator.com");
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
console.log("importing data to http://localhost:8080");
const importResp = await fetch("http://localhost:8080/data/import", {
  headers: {
    authorization: `Bearer ${SERVICE_ROLE_KEY}`,
    "content-type": "application/json",
  },
  body: JSON.stringify(data),
  method: "POST",
});
if (importResp.status !== 200) {
  console.error(await importResp.text());
  Deno.exit(1);
}
