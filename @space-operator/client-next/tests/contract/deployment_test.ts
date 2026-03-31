import { assert, assertEquals } from "@std/assert";
import {
  bearerAuth,
  createClient,
  publicKeyAuth,
  signAndSubmitSignature,
} from "../../src/mod.ts";
import { decodeBase58, decodeBase64, encodeBase58 } from "../../src/deps.ts";
import {
  adminSupabase,
  apiClient,
  checkNoErrorsAdmin,
  contractTest,
  FLOW_SERVER_URL,
  getEnv,
  ownerUserId,
  resolveFixtureFlowId,
  serviceInfo,
  signText,
  Value,
  web3,
} from "./_shared.ts";

contractTest(
  "deployment contract: deploy, start, sign, and fetch output",
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

    const request = await run.signatureRequest();
    assertEquals(request.pubkey, starterKeypair.publicKey.toBase58());
    await signAndSubmitSignature(starter.signatures, request, {
      publicKey: starterKeypair.publicKey,
      signTransaction: (tx: web3.VersionedTransaction) => {
        tx.sign([starterKeypair]);
        return tx;
      },
    });

    const output = await run.output();
    assert(output.M?.signature != null);
  },
);

contractTest(
  "deployment contract: authenticated starts by flow still work",
  async () => {
    const owner = apiClient();
    const deploySimpleFlowId = await resolveFixtureFlowId("deploySimple");
    const deploymentId = await owner.flows.deploy(deploySimpleFlowId);
    const admin = adminSupabase();
    const updateResult = await admin
      .from("flow_deployments")
      .update({
        start_permission: "Authenticated",
      })
      .eq("id", deploymentId);
    if (updateResult.error) {
      throw new Error(JSON.stringify(updateResult.error));
    }

    const starterKeypair = web3.Keypair.generate();
    const { anon_key } = await serviceInfo();
    const starter = createClient({
      baseUrl: FLOW_SERVER_URL,
      anonKey: anon_key,
    });
    const auth = await starter.auth.loginWithSignature(
      starterKeypair.publicKey,
      (message) => signText(starterKeypair, message),
    );
    const authedStarter = starter.withAuth(
      bearerAuth(auth.session.access_token),
    );
    const run = await authedStarter.deployments.start(
      { flow: deploySimpleFlowId },
      {
        inputs: {
          a: 1,
          b: 2,
        },
      },
    );
    const output = await run.output();
    const selectResult = await admin
      .from("flow_run")
      .select("output")
      .eq("deployment_id", deploymentId)
      .single();
    if (selectResult.error) {
      throw new Error(JSON.stringify(selectResult.error));
    }

    assertEquals(output.toJSObject().c, 3);
    assertEquals(
      output,
      Value.fromJSON(
        selectResult.data.output as Parameters<typeof Value.fromJSON>[0],
      ),
    );
    await checkNoErrorsAdmin(run.id);
  },
);

contractTest(
  "deployment contract: start by flow and latest tag still works",
  async () => {
    const owner = apiClient();
    const deploySimpleFlowId = await resolveFixtureFlowId("deploySimple");
    await owner.flows.deploy(deploySimpleFlowId);
    const run = await owner.deployments.start(
      {
        flow: deploySimpleFlowId,
        tag: "latest",
      },
      {
        inputs: {
          a: 1,
          b: 2,
        },
      },
    );
    const output = await run.output();

    assertEquals(output.toJSObject().c, 3);
  },
);

contractTest("deployment contract: custom tags still resolve", async () => {
  const owner = apiClient();
  const deploySimpleFlowId = await resolveFixtureFlowId("deploySimple");
  const supabaseClient = adminSupabase();
  const userId = await ownerUserId();

  const deploymentId = await owner.flows.deploy(deploySimpleFlowId);
  const upsertResult = await supabaseClient.from("flow_deployments_tags")
    .upsert({
      deployment_id: deploymentId,
      entrypoint: deploySimpleFlowId,
      tag: "v1",
      user_id: userId,
    });
  if (upsertResult.error) {
    throw new Error(JSON.stringify(upsertResult.error));
  }

  const run = await owner.deployments.start(
    {
      flow: deploySimpleFlowId,
      tag: "v1",
    },
    {
      inputs: {
        a: 1,
        b: 2,
      },
    },
  );
  const output = await run.output();

  assertEquals(output.toJSObject().c, 3);
});

contractTest(
  "deployment contract: delete still rolls latest tags back",
  async () => {
    const owner = apiClient();
    const deployDeleteFlowId = await resolveFixtureFlowId("deployDelete");
    const supabaseClient = adminSupabase();

    const cleanup = await supabaseClient
      .from("flow_deployments")
      .delete()
      .eq("entrypoint", deployDeleteFlowId);
    if (cleanup.error) {
      throw new Error(JSON.stringify(cleanup.error));
    }

    const getLatest = async (): Promise<string> => {
      const result = await supabaseClient
        .from("flow_deployments_tags")
        .select("deployment_id")
        .eq("tag", "latest")
        .eq("entrypoint", deployDeleteFlowId)
        .single();
      if (result.error) {
        throw new Error(JSON.stringify(result.error));
      }
      return result.data.deployment_id;
    };

    const first = await owner.flows.deploy(deployDeleteFlowId);
    assertEquals(await getLatest(), first);

    const second = await owner.flows.deploy(deployDeleteFlowId);
    assertEquals(await getLatest(), second);

    const deleteSecond = await supabaseClient
      .from("flow_deployments")
      .delete({ count: "exact" })
      .eq("id", second);
    if (deleteSecond.error) {
      throw new Error(JSON.stringify(deleteSecond.error));
    }
    assertEquals(deleteSecond.count, 1);
    assertEquals(await getLatest(), first);

    const count = 10;
    for (let i = 0; i < count; i += 1) {
      await owner.flows.deploy(deployDeleteFlowId);
    }

    const deleteBatch = await supabaseClient
      .from("flow_deployments")
      .delete({ count: "exact" })
      .eq("entrypoint", deployDeleteFlowId)
      .neq("id", first);
    if (deleteBatch.error) {
      throw new Error(JSON.stringify(deleteBatch.error));
    }
    assertEquals(deleteBatch.count, count);
    assertEquals(await getLatest(), first);
  },
);

contractTest(
  "deployment contract: output instructions still return a transaction",
  async () => {
    const owner = apiClient();
    const deployActionFlowId = await resolveFixtureFlowId("deployAction");
    const supabaseClient = adminSupabase();

    const deploymentId = await owner.flows.deploy(deployActionFlowId);
    const updateResult = await supabaseClient
      .from("flow_deployments")
      .update({
        output_instructions: true,
        start_permission: "Anonymous",
      })
      .eq("id", deploymentId);
    if (updateResult.error) {
      throw new Error(JSON.stringify(updateResult.error));
    }

    const starterKeypair = web3.Keypair.generate();
    const starter = owner.withAuth(publicKeyAuth(starterKeypair.publicKey));
    const run = await starter.deployments.start(
      { id: deploymentId },
      {
        inputs: {
          sender: starterKeypair.publicKey,
        },
      },
    );
    const output = await run.output();
    const text = output.toJSObject().transaction;
    const tx = web3.VersionedTransaction.deserialize(decodeBase64(text));
    const msg = web3.TransactionMessage.decompile(tx.message);
    const transfer = web3.SystemInstruction.decodeTransfer(msg.instructions[2]);

    assertEquals(transfer.fromPubkey, starterKeypair.publicKey);
    await checkNoErrorsAdmin(run.id);
  },
);

contractTest(
  "deployment contract: fees still appear in output instructions",
  async () => {
    const owner = apiClient();
    const deployActionFlowId = await resolveFixtureFlowId("deployAction");
    const supabaseClient = adminSupabase();

    const deploymentId = await owner.flows.deploy(deployActionFlowId);
    const feeRecipient = new web3.PublicKey(
      "J8mdVB7duENExHwKgyHnK3gve8CvUgFsmwWkJ55LWgZj",
    );
    const feeAmount = 1_000_000;
    const updateResult = await supabaseClient
      .from("flow_deployments")
      .update({
        output_instructions: true,
        start_permission: "Anonymous",
        fees: [[feeRecipient.toBase58(), feeAmount]],
      })
      .eq("id", deploymentId);
    if (updateResult.error) {
      throw new Error(JSON.stringify(updateResult.error));
    }

    const starterKeypair = web3.Keypair.generate();
    const starter = owner.withAuth(publicKeyAuth(starterKeypair.publicKey));
    const run = await starter.deployments.start(
      {
        flow: deployActionFlowId,
        tag: "latest",
      },
      {
        inputs: {
          sender: starterKeypair.publicKey,
        },
      },
    );
    const output = await run.output();
    const text = output.toJSObject().transaction;
    const tx = web3.VersionedTransaction.deserialize(decodeBase64(text));
    const msg = web3.TransactionMessage.decompile(tx.message);
    const transfer = web3.SystemInstruction.decodeTransfer(msg.instructions[2]);
    const fee = web3.SystemInstruction.decodeTransfer(msg.instructions[3]);

    assertEquals(transfer.fromPubkey, starterKeypair.publicKey);
    assertEquals(fee.fromPubkey, starterKeypair.publicKey);
    assertEquals(fee.toPubkey, feeRecipient);
    assertEquals(fee.lamports, BigInt(feeAmount));
    await checkNoErrorsAdmin(run.id);
  },
);

contractTest(
  "deployment contract: action identity still appears in output instructions",
  async () => {
    const owner = apiClient();
    const deployActionFlowId = await resolveFixtureFlowId("deployAction");
    const supabaseClient = adminSupabase();

    const deploymentId = await owner.flows.deploy(deployActionFlowId);
    const identity = web3.Keypair.generate();
    const userId = await ownerUserId();

    const ownerBearer = createClient({
      baseUrl: FLOW_SERVER_URL,
      auth: bearerAuth((await owner.auth.claimToken()).access_token),
    });
    await ownerBearer.wallets.upsert({
      type: "HARDCODED",
      name: "identity",
      public_key: identity.publicKey.toBase58(),
      keypair: encodeBase58(identity.secretKey),
      user_id: userId,
    });
    const updateResult = await supabaseClient
      .from("flow_deployments")
      .update({
        output_instructions: true,
        start_permission: "Anonymous",
        action_identity: identity.publicKey.toBase58(),
      })
      .eq("id", deploymentId);
    if (updateResult.error) {
      throw new Error(JSON.stringify(updateResult.error));
    }

    const starterKeypair = web3.Keypair.generate();
    const starter = owner.withAuth(publicKeyAuth(starterKeypair.publicKey));
    const run = await starter.deployments.start(
      {
        flow: deployActionFlowId,
        tag: "latest",
      },
      {
        inputs: {
          sender: starterKeypair.publicKey,
        },
      },
    );
    const output = await run.output();
    const text = output.toJSObject().transaction;
    const tx = web3.VersionedTransaction.deserialize(decodeBase64(text));
    const msg = web3.TransactionMessage.decompile(tx.message);
    const transfer = web3.SystemInstruction.decodeTransfer(msg.instructions[2]);
    const memo = msg.instructions[3].data.toString("utf-8");

    assertEquals(transfer.fromPubkey, starterKeypair.publicKey);
    assert(memo.startsWith(`solana-action:${identity.publicKey.toBase58()}`));
    await checkNoErrorsAdmin(run.id);
  },
);

contractTest(
  "deployment contract: execute on action still works with action_signer",
  async () => {
    const owner = apiClient();
    const deployActionFlowId = await resolveFixtureFlowId("deployAction");
    const supabaseClient = adminSupabase();
    const userId = await ownerUserId();

    const deploymentId = await owner.flows.deploy(deployActionFlowId);
    const identity = web3.Keypair.generate();
    const ownerBearer = createClient({
      baseUrl: FLOW_SERVER_URL,
      auth: bearerAuth((await owner.auth.claimToken()).access_token),
    });
    await ownerBearer.wallets.upsert({
      type: "HARDCODED",
      name: "identity",
      public_key: identity.publicKey.toBase58(),
      keypair: encodeBase58(identity.secretKey),
      user_id: userId,
    });
    const updateResult = await supabaseClient
      .from("flow_deployments")
      .update({
        start_permission: "Anonymous",
        action_identity: identity.publicKey.toBase58(),
      })
      .eq("id", deploymentId);
    if (updateResult.error) {
      throw new Error(JSON.stringify(updateResult.error));
    }

    const starterKeypair = web3.Keypair.generate();
    const starter = owner.withAuth(publicKeyAuth(starterKeypair.publicKey));

    const connection = new web3.Connection(getEnv("SOLANA_DEVNET_URL"));
    const ownerKeypair = web3.Keypair.fromSecretKey(
      decodeBase58(getEnv("KEYPAIR")),
    );
    const recent = await connection.getLatestBlockhash();
    const fundTx = web3.Transaction.populate(
      web3.Message.compile({
        instructions: [
          web3.SystemProgram.transfer({
            fromPubkey: ownerKeypair.publicKey,
            toPubkey: starterKeypair.publicKey,
            lamports: 0.1 * web3.LAMPORTS_PER_SOL + 10_000,
          }),
        ],
        payerKey: ownerKeypair.publicKey,
        recentBlockhash: recent.blockhash,
      }),
    );
    await web3.sendAndConfirmTransaction(connection, fundTx, [ownerKeypair]);

    const run = await starter.deployments.start(
      {
        flow: deployActionFlowId,
        tag: "latest",
      },
      {
        inputs: {
          sender: starterKeypair.publicKey,
        },
        action_signer: starterKeypair.publicKey.toBase58(),
      },
    );
    const request = await run.signatureRequest();
    const tx = request.buildTransaction();
    tx.sign([starterKeypair]);
    const signature = await connection.sendTransaction(tx, {
      skipPreflight: true,
    });

    const output = await run.output();
    assertEquals(decodeBase58(signature), output.toJSObject().signature);
    await checkNoErrorsAdmin(run.id);
  },
);
