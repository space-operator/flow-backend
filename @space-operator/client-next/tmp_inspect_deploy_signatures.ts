import { load } from "@std/dotenv";
await load({ export: true, envPath: decodeURIComponent(new URL("../../.env", import.meta.url).pathname) });
import { createClient, apiKeyAuth, publicKeyAuth, web3 } from "./src/mod.ts";
import { decodeBase58 } from "./src/deps.ts";
import { resolveFixtureFlowId } from "./tests/contract/_shared.ts";

const FLOW_SERVER_URL = Deno.env.get("FLOW_SERVER_URL") ?? "http://localhost:8080";
const apiKey = Deno.env.get("APIKEY");
const secret = Deno.env.get("KEYPAIR") ?? Deno.env.get("keypair") ?? Deno.env.get("OWNER_KEYPAIR");
if (!apiKey || !secret) throw new Error("missing APIKEY/KEYPAIR");
const owner = createClient({ baseUrl: FLOW_SERVER_URL, auth: apiKeyAuth(apiKey) });
const ownerKeypair = web3.Keypair.fromSecretKey(decodeBase58(secret));
const deployRunFlowId = await resolveFixtureFlowId("deployRun");
console.log("deployRunFlowId", deployRunFlowId);
const deploymentId = await owner.flows.deploy(deployRunFlowId);
console.log("deploymentId", deploymentId);
const starterKeypair = web3.Keypair.generate();
console.log("owner", ownerKeypair.publicKey.toBase58());
console.log("starter", starterKeypair.publicKey.toBase58());
const starter = owner.withAuth(publicKeyAuth(starterKeypair.publicKey));
const run = await starter.deployments.start({ id: deploymentId }, { inputs: { sender: starterKeypair.publicKey, n: 2 } });
console.log("run", run.id, "token?", !!run.token);

const ownerReqPromise = owner.flows.signatureRequest(run.id, { timeoutMs: 20000 }).then((req) => ({ who: "owner", req })).catch((error) => ({ who: "owner", error }));
const runReqPromise = run.signatureRequest({ timeoutMs: 20000 }).then((req) => ({ who: "run", req })).catch((error) => ({ who: "run", error }));
const results = await Promise.all([ownerReqPromise, runReqPromise]);
for (const result of results) {
  if ("req" in result) {
    console.log(result.who, "request", result.req.id, result.req.pubkey);
  } else {
    console.log(result.who, "error", String(result.error));
  }
}
