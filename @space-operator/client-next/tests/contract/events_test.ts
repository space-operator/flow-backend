import { assert, assertEquals } from "@std/assert";
import { publicKeyAuth } from "../../src/mod.ts";
import {
  apiClient,
  contractTest,
  resolveFixtureFlowId,
  web3,
} from "./_shared.ts";

contractTest(
  "events contract: signature request subscriptions emit owner requests",
  async () => {
    const owner = apiClient();
    const deployRunFlowId = await resolveFixtureFlowId("deployRun");
    const ownerSession = await owner.auth.claimToken();
    const deploymentId = await owner.flows.deploy(deployRunFlowId);
    const ws = owner.ws();
    await ws.authenticate();
    assertEquals(ws.getIdentity()?.user_id, ownerSession.user_id);

    const subscription = await ws.subscribeSignatureRequests({
      signal: AbortSignal.timeout(30_000),
    });
    const starterKeypair = web3.Keypair.generate();
    const starter = owner.withAuth(publicKeyAuth(starterKeypair.publicKey));
    const run = await starter.deployments.start(
      { id: deploymentId },
      {
        inputs: {
          sender: starterKeypair.publicKey,
          n: 2,
        },
      },
    );

    try {
      let requestId: number | undefined;

      for await (const event of subscription) {
        if (event.data.flow_run_id === run.id) {
          requestId = event.data.id;
          break;
        }
      }

      assert(requestId != null);
    } finally {
      await subscription.close();
      await ws.close();
      await run.stop({ reason: "events contract cleanup" }).catch(() =>
        undefined
      );
    }
  },
);
