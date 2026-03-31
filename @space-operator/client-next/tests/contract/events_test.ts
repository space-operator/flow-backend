import { assert } from "@std/assert";
import { publicKeyAuth } from "../../src/mod.ts";
import {
  apiClient,
  contractTest,
  resolveFixtureFlowId,
  web3,
} from "./_shared.ts";

contractTest(
  "events contract: flow-run subscriptions emit deployment signature requests",
  async () => {
    const owner = apiClient();
    const deployRunFlowId = await resolveFixtureFlowId("deployRun");
    const deploymentId = await owner.flows.deploy(deployRunFlowId);
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
    const subscription = await run.events({
      signal: AbortSignal.timeout(30_000),
    });

    try {
      let signaturePubkey: string | undefined;

      for await (const event of subscription) {
        if (event.event === "SignatureRequest") {
          signaturePubkey = event.data.pubkey;
          break;
        }
      }

      assert(signaturePubkey != null);
      assert(signaturePubkey.length > 0);
    } finally {
      await subscription.close();
      await run.stop({ reason: "events contract cleanup" }).catch(() =>
        undefined
      );
    }
  },
);
