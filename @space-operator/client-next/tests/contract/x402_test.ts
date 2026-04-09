import { assertEquals } from "@std/assert";
import { createKeyPairSignerFromBytes } from "@solana/kit";
import { createClient, publicKeyAuth } from "../../src/mod.ts";
import { wrapFetchWithPayment } from "../../src/x402/mod.ts";
import {
  adminSupabase,
  apiClient,
  contractTest,
  FLOW_SERVER_URL,
  getEnv,
  ownerUserId,
  resolveFixtureFlowId,
  RUN_X402_TESTS,
  web3,
} from "./_shared.ts";

contractTest("x402 contract: start deployment with wrapped fetch", async () => {
  const owner = apiClient();
  const x402FlowId = await resolveFixtureFlowId("x402");
  const deploymentId = await owner.flows.deploy(x402FlowId);
  const admin = adminSupabase();
  const userId = await ownerUserId();
  const { decodeBase58 } = await import("../../src/deps.ts");
  const signer = await createKeyPairSignerFromBytes(
    decodeBase58(getEnv("KEYPAIR")),
  );
  const ownerKeypair = web3.Keypair.fromSecretKey(decodeBase58(getEnv("KEYPAIR")));
  const walletResult = await admin
    .from("wallets")
    .select("id")
    .eq("user_id", userId)
    .eq("public_key", ownerKeypair.publicKey.toBase58())
    .limit(1)
    .maybeSingle();
  if (walletResult.error) {
    throw new Error(JSON.stringify(walletResult.error));
  }
  if (walletResult.data?.id == null) {
    throw new Error(
      `could not find owner wallet ${ownerKeypair.publicKey.toBase58()} for x402 fees`,
    );
  }

  const startPermissionUpdate = await admin
    .from("flow_deployments")
    .update({ start_permission: "Anonymous" })
    .eq("id", deploymentId);
  if (startPermissionUpdate.error) {
    throw new Error(JSON.stringify(startPermissionUpdate.error));
  }

  const feeInsert = await admin.from("flow_deployments_x402_fees").insert({
    user_id: userId,
    deployment_id: deploymentId,
    amount: 0.01,
    enabled: true,
    network: "solana-devnet",
    pay_to: walletResult.data.id,
  });
  if (feeInsert.error) {
    throw new Error(JSON.stringify(feeInsert.error));
  }

  const starterKeypair = web3.Keypair.generate();
  const fetchWithPayment = wrapFetchWithPayment(fetch, signer);
  const client = createClient({
    baseUrl: FLOW_SERVER_URL,
    auth: publicKeyAuth(starterKeypair.publicKey),
    fetch: fetchWithPayment,
  });

  try {
    const run = await client.deployments.start(
      { id: deploymentId },
      { inputs: { n: 2 } },
    );
    const output = await run.output();
    assertEquals(output.toJSObject().out, 1);
  } finally {
    const deleteResult = await admin
      .from("flow_deployments")
      .delete()
      .eq("id", deploymentId);
    if (deleteResult.error) {
      throw new Error(JSON.stringify(deleteResult.error));
    }
  }
}, {
  ignore: !RUN_X402_TESTS,
});
