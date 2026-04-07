import { assert, assertEquals, assertRejects } from "@std/assert";
import {
  ApiError,
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
  RUN_READ_E2E_TESTS,
  resolveFixtureFlowId,
  serviceInfo,
  signText,
  SUPABASE_URL,
  Value,
  web3,
} from "./_shared.ts";

type FlowV2Flags = {
  read_enabled?: boolean;
};

type FlowRunRow = {
  id: string;
  origin: Record<string, unknown>;
  start_time: string | null;
  end_time: string | null;
};

async function requestFlowV2<T>(
  method: "GET" | "PATCH",
  query: string,
  body?: FlowV2Flags,
): Promise<T> {
  const serviceRoleKey = getEnv("SERVICE_ROLE_KEY");
  const response = await fetch(`${SUPABASE_URL}/rest/v1/flows_v2${query}`, {
    method,
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
      ...(body ? { "content-type": "application/json" } : {}),
      ...(method === "PATCH" ? { prefer: "return=representation" } : {}),
    },
    ...(body ? { body: JSON.stringify(body) } : {}),
  });
  const text = await response.text();
  if (!response.ok) {
    throw new Error(
      `flows_v2 ${method} failed: ${response.status} ${text || response.statusText}`,
    );
  }
  return text.length === 0 ? [] as T : JSON.parse(text) as T;
}

async function requestFlowRuns<T>(query: string): Promise<T> {
  const serviceRoleKey = getEnv("SERVICE_ROLE_KEY");
  const response = await fetch(`${SUPABASE_URL}/rest/v1/flow_run${query}`, {
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
    },
  });
  const text = await response.text();
  if (!response.ok) {
    throw new Error(
      `flow_run GET failed: ${response.status} ${text || response.statusText}`,
    );
  }
  return text.length === 0 ? [] as T : JSON.parse(text) as T;
}

async function withFlowReadEnabled(
  flowId: string,
  readEnabled: boolean,
  fn: () => Promise<void>,
) {
  const query = `?uuid=eq.${flowId}&select=uuid,read_enabled`;
  const [original] = await requestFlowV2<Array<{
    uuid: string;
    read_enabled: boolean;
  }>>("GET", query);
  if (!original) {
    throw new Error(`missing flows_v2 row for ${flowId}`);
  }

  await requestFlowV2("PATCH", query, {
    read_enabled: readEnabled,
  });

  try {
    await fn();
  } finally {
    await requestFlowV2("PATCH", query, {
      read_enabled: original.read_enabled,
    });
  }
}

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
  "deployment contract: read requires read_enabled at deploy time",
  async () => {
    const owner = apiClient();
    const deploySimpleFlowId = await resolveFixtureFlowId("deploySimple");

    await withFlowReadEnabled(
      deploySimpleFlowId,
      false,
      async () => {
        const deploymentId = await owner.flows.deploy(deploySimpleFlowId);
        const error = await assertRejects(
          () =>
            owner.deployments.read(
              { id: deploymentId },
              {
                inputs: {
                  a: 2,
                  b: 3,
                },
                skipCache: true,
              },
            ),
          ApiError,
        );
        assertEquals(error.status, 403);
      },
    );

    await withFlowReadEnabled(
      deploySimpleFlowId,
      true,
      async () => {
        const deploymentId = await owner.flows.deploy(deploySimpleFlowId);
        const result = await owner.deployments.read(
          { id: deploymentId },
          {
            inputs: {
              a: 2,
              b: 3,
            },
            skipCache: true,
          },
        );

        assertEquals(result.value.toJSObject().c, 5);
      },
    );
  },
  { ignore: !RUN_READ_E2E_TESTS },
);

contractTest(
  "deployment contract: repeated reads stay under budget and collapse to one read run",
  async () => {
    const owner = apiClient();
    const deploySimpleFlowId = await resolveFixtureFlowId("deploySimple");

    await withFlowReadEnabled(
      deploySimpleFlowId,
      true,
      async () => {
        const deploymentId = await owner.flows.deploy(deploySimpleFlowId);
        const startedAt = new Date();
        const perRequestMs: number[] = [];

        const wallStart = performance.now();
        for (let i = 0; i < 10; i++) {
          const requestStart = performance.now();
          const result = await apiClient().deployments.read(
            { id: deploymentId },
            {
              inputs: {
                a: 2,
                b: 3,
              },
            },
          );
          perRequestMs.push(performance.now() - requestStart);
          assertEquals(result.value.toJSObject().c, 5);
        }
        const elapsedMs = performance.now() - wallStart;

        const runs = await requestFlowRuns<FlowRunRow[]>(
          `?deployment_id=eq.${deploymentId}&start_time=gte.${
            encodeURIComponent(startedAt.toISOString())
          }&select=id,origin,start_time,end_time&order=start_time.asc`,
        );
        const readRuns = runs.filter((row) =>
          row.origin != null && Object.hasOwn(row.origin, "Read")
        );

        assert(
          elapsedMs <= 2_000,
          `expected 10 deployment reads in <= 2000ms, got ${
            elapsedMs.toFixed(2)
          }ms`,
        );
        assertEquals(
          readRuns.length,
          1,
          `expected one persisted deployment read run, got ${readRuns.length}; per-request timings: ${
            perRequestMs.map((ms) => ms.toFixed(2)).join(", ")
          }`,
        );
        assert(
          readRuns[0].end_time != null,
          "expected cached deployment read run to finish",
        );
      },
    );
  },
  { ignore: !RUN_READ_E2E_TESTS },
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
