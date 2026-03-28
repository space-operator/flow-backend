import { assertEquals } from "@std/assert";
import { createKeyPairSignerFromBytes } from "@solana/kit";
import { createClient, publicKeyAuth } from "../../src/mod.ts";
import { wrapFetchWithPayment } from "../../src/x402/mod.ts";
import {
  apiClient,
  contractTest,
  FLOW_SERVER_URL,
  getEnv,
  resolveFixtureFlowId,
  web3,
} from "./_shared.ts";

contractTest("x402 contract: start deployment with wrapped fetch", async () => {
  const owner = apiClient();
  const x402FlowId = await resolveFixtureFlowId("x402");
  const deploymentId = await owner.flows.deploy(x402FlowId);
  const { decodeBase58 } = await import("../../src/deps.ts");
  const signer = await createKeyPairSignerFromBytes(
    decodeBase58(getEnv("KEYPAIR")),
  );
  const starterKeypair = web3.Keypair.generate();
  const fetchWithPayment = wrapFetchWithPayment(fetch, signer);
  const client = createClient({
    baseUrl: FLOW_SERVER_URL,
    auth: publicKeyAuth(starterKeypair.publicKey),
    fetch: fetchWithPayment,
  });

  const run = await client.deployments.start(
    { id: deploymentId },
    { inputs: { n: 2 } },
  );
  const output = await run.output();

  assertEquals(output.toJSObject().out, 1);
});
