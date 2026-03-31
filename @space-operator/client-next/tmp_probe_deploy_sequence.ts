import { load } from "@std/dotenv";
await load({ export: true, envPath: decodeURIComponent(new URL("../../.env", import.meta.url).pathname) });
import { createClient, apiKeyAuth, publicKeyAuth, signAndSubmitSignature, web3 } from "./src/mod.ts";
import { decodeBase58 } from "./src/deps.ts";
import { resolveFixtureFlowId } from "./tests/contract/_shared.ts";

const FLOW_SERVER_URL = Deno.env.get("FLOW_SERVER_URL") ?? "http://localhost:8080";
const apiKey = Deno.env.get("APIKEY")!;
const secret = Deno.env.get("KEYPAIR") ?? Deno.env.get("keypair") ?? Deno.env.get("OWNER_KEYPAIR");
if (!secret) throw new Error("missing KEYPAIR");
const owner = createClient({ baseUrl: FLOW_SERVER_URL, auth: apiKeyAuth(apiKey) });
const ownerKeypair = web3.Keypair.fromSecretKey(decodeBase58(secret));
const deployRunFlowId = await resolveFixtureFlowId("deployRun");
const deploymentId = await owner.flows.deploy(deployRunFlowId);
const ownerWs = owner.ws();
await ownerWs.authenticate();
const sub = await ownerWs.subscribeSignatureRequests({ signal: AbortSignal.timeout(20000) });
const starterKeypair = web3.Keypair.generate();
const starter = owner.withAuth(publicKeyAuth(starterKeypair.publicKey));
const run = await starter.deployments.start({ id: deploymentId }, { inputs: { sender: starterKeypair.publicKey, n: 2 } });
console.log("run", run.id, "owner", ownerKeypair.publicKey.toBase58(), "starter", starterKeypair.publicKey.toBase58());
const first = await run.signatureRequest({ timeoutMs: 20000 });
console.log("first request", first.id, first.pubkey);
await signAndSubmitSignature(starter.signatures, first, {
  publicKey: starterKeypair.publicKey,
  signTransaction: (tx) => {
    tx.sign([starterKeypair]);
    return tx;
  },
});
console.log("signed first");
const secondOwner = await owner.flows.signatureRequest(run.id, { timeoutMs: 20000 }).then((req) => ({ req })).catch((error) => ({ error }));
console.log("second via owner", JSON.stringify("req" in secondOwner ? { id: secondOwner.req.id, pubkey: secondOwner.req.pubkey } : { error: String(secondOwner.error) }));
const secondRun = await run.signatureRequest({ timeoutMs: 20000 }).then((req) => ({ req })).catch((error) => ({ error }));
console.log("second via run", JSON.stringify("req" in secondRun ? { id: secondRun.req.id, pubkey: secondRun.req.pubkey } : { error: String(secondRun.error) }));
const nextEvent = await sub.next().then((x) => ({ x })).catch((error) => ({ error }));
console.log("ws event", JSON.stringify("x" in nextEvent ? { done: nextEvent.x.done, id: nextEvent.x.value?.data.id, pubkey: nextEvent.x.value?.data.pubkey } : { error: String(nextEvent.error) }));
await sub.close().catch(() => undefined);
await ownerWs.close().catch(() => undefined);
