import { load } from "@std/dotenv";
await load({ export: true, envPath: decodeURIComponent(new URL("../../.env", import.meta.url).pathname) });
import { createClient, apiKeyAuth, publicKeyAuth, signAndSubmitSignature, web3 } from "./src/mod.ts";
import { resolveFixtureFlowId } from "./tests/contract/_shared.ts";

const FLOW_SERVER_URL = Deno.env.get("FLOW_SERVER_URL") ?? "http://localhost:8080";
const owner = createClient({ baseUrl: FLOW_SERVER_URL, auth: apiKeyAuth(Deno.env.get("APIKEY")!) });
const deployRunFlowId = await resolveFixtureFlowId("deployRun");
const deploymentId = await owner.flows.deploy(deployRunFlowId);
const starterKeypair = web3.Keypair.generate();
const starter = owner.withAuth(publicKeyAuth(starterKeypair.publicKey));
const run = await starter.deployments.start({ id: deploymentId }, { inputs: { sender: starterKeypair.publicKey, n: 2 } });
const req = await run.signatureRequest({ timeoutMs: 20000 });
console.log("first", req.pubkey);
await signAndSubmitSignature(starter.signatures, req, {
  publicKey: starterKeypair.publicKey,
  signTransaction: (tx) => {
    tx.sign([starterKeypair]);
    return tx;
  },
});
const output = await run.output({ timeoutMs: 30000 });
console.log(JSON.stringify(output.toJSObject()));
