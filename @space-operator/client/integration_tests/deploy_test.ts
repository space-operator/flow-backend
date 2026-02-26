import { bs58, Value, web3 } from "../src/deps.ts";
import * as client from "../src/mod.ts";
import * as dotenv from "@std/dotenv";
import { createClient } from "@supabase/supabase-js";
import { assert, assertEquals } from "@std/assert";
import { LAMPORTS_PER_SOL } from "@solana/web3.js";
import * as nacl from "tweetnacl";
import { decodeBase64 } from "@std/encoding/base64";
import { checkNoErrors, getEnv } from "./utils.ts";
import { encodeBase58 } from "@std/encoding/base58";

dotenv.loadSync({
  export: true,
});

function ed25519SignText(keypair: web3.Keypair, message: string): Uint8Array {
  return nacl.default.sign.detached(
    new TextEncoder().encode(message),
    keypair.secretKey,
  );
}

const anonKey = getEnv("ANON_KEY");
const apiKey = getEnv("APIKEY");
const supabaseUrl = "http://localhost:8000";
const DEPLOY_RUN_FLOW_ID = "92b480ad-1a18-4a52-a459-4d5420890272"; // Transfer SOL
const DEPLOY_DELETE_FLOW_ID = "102244df-74aa-4f77-a556-d9d279c64655"; // Collatz-Core
const DEPLOY_ACTION_FLOW_ID = "9647ba16-de20-4209-9056-1a3dd8c2d6ab"; // Simple Transfer
const DEPLOY_SIMPLE_FLOW_ID = "9647ba16-de20-4209-9056-1a3dd8c2d6ab"; // Simple Transfer

Deno.test("deploy and run", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });
  const ownerKeypair = web3.Keypair.fromSecretKey(
    bs58.decodeBase58(getEnv("KEYPAIR")),
  );

  const id = await owner.deployFlow(DEPLOY_RUN_FLOW_ID);

  const starterKeypair = web3.Keypair.generate();
  const starter = new client.Client({
    host: "http://localhost:8080",
    anonKey,
  });
  starter.setToken(starterKeypair.publicKey.toString());
  const { flow_run_id, token } = await starter.startDeployment(
    {
      id,
    },
    {
      inputs: new Value({
        sender: starterKeypair.publicKey,
        n: 2,
      }).M!,
    },
  );
  starter.setToken(token);

  {
    const req = await owner.getSignatureRequest(flow_run_id);
    await owner.signAndSubmitSignature(
      req,
      ownerKeypair.publicKey,
      (tx) => {
        tx.sign([ownerKeypair]);
        return tx;
      },
    );
  }

  {
    const req = await starter.getSignatureRequest(flow_run_id);
    await starter.signAndSubmitSignature(
      req,
      starterKeypair.publicKey,
      (tx) => {
        tx.sign([starterKeypair]);
        return tx;
      },
    );
  }

  const result = await starter.getFlowOutput(flow_run_id);
  const signature = result.M?.signature.asBytes();
  assert(signature != null);

  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);
  await checkNoErrors(sup, flow_run_id);
});

Deno.test("deploy and delete", async (t) => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });
  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);

  const getLatest = async (
    flowId: client.FlowId,
  ): Promise<client.DeploymentId> => {
    const result = await sup
      .from("flow_deployments_tags")
      .select("deployment_id")
      .eq("tag", "latest")
      .eq("entrypoint", flowId)
      .single();
    if (result.error) throw new Error(JSON.stringify(result.error));
    return result.data.deployment_id;
  };

  const first = await owner.deployFlow(DEPLOY_DELETE_FLOW_ID);
  const firstFromTag = await getLatest(DEPLOY_DELETE_FLOW_ID);
  await t.step("assert first", () => {
    assertEquals(first, firstFromTag);
  });

  const second = await owner.deployFlow(DEPLOY_DELETE_FLOW_ID);
  const secondFromTag = await getLatest(DEPLOY_DELETE_FLOW_ID);
  await t.step("assert second", () => {
    assertEquals(second, secondFromTag);
  });

  await t.step("delete", async () => {
    const deleteResult = await sup
      .from("flow_deployments")
      .delete({ count: "exact" })
      .eq("id", second);
    if (deleteResult.error) throw new Error(JSON.stringify(deleteResult.error));

    assertEquals(deleteResult.count, 1);
  });

  await t.step("assert after delete", async () => {
    const currentLatest = await getLatest(DEPLOY_DELETE_FLOW_ID);
    assertEquals(first, currentLatest);
  });

  const count = 10;
  for (let i = 0; i < count; i += 1) {
    await owner.deployFlow(DEPLOY_DELETE_FLOW_ID);
  }

  await t.step("batch delete", async () => {
    const deleteResult = await sup
      .from("flow_deployments")
      .delete({ count: "exact" })
      .eq("entrypoint", DEPLOY_DELETE_FLOW_ID)
      .neq("id", first);
    if (deleteResult.error) throw new Error(JSON.stringify(deleteResult.error));
    assertEquals(deleteResult.count, count);
  });

  await t.step("assert after batch", async () => {
    const currentLatest = await getLatest(DEPLOY_DELETE_FLOW_ID);
    assertEquals(first, currentLatest);
  });
});

Deno.test("output instructions", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });
  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);

  const flowId = DEPLOY_ACTION_FLOW_ID;
  const id = await owner.deployFlow(flowId);

  const updateResult = await sup
    .from("flow_deployments")
    .update({
      output_instructions: true,
      start_permission: "Anonymous",
    })
    .eq("id", id);
  if (updateResult.error) throw new Error(JSON.stringify(updateResult.error));

  const starterKeypair = web3.Keypair.generate();
  const starter = new client.Client({
    host: "http://localhost:8080",
    anonKey,
  });
  starter.setToken(starterKeypair.publicKey.toString());
  const { flow_run_id, token } = await starter.startDeployment(
    {
      id,
    },
    {
      inputs: new Value({
        sender: starterKeypair.publicKey,
      }).M!,
    },
  );
  starter.setToken(token);
  const output = await starter.getFlowOutput(flow_run_id);
  const text = output.toJSObject().transaction;
  await checkNoErrors(sup, flow_run_id);
  const tx = web3.VersionedTransaction.deserialize(decodeBase64(text));
  const msg = web3.TransactionMessage.decompile(tx.message);
  const transfer = web3.SystemInstruction.decodeTransfer(msg.instructions[2]);
  assertEquals(transfer.fromPubkey, starterKeypair.publicKey);
});

Deno.test("fees", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });
  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);

  const flowId = DEPLOY_ACTION_FLOW_ID;
  const id = await owner.deployFlow(flowId);
  const feeRecipient = new web3.PublicKey(
    "J8mdVB7duENExHwKgyHnK3gve8CvUgFsmwWkJ55LWgZj",
  );
  const feeAmount = 1000000;
  const updateResult = await sup
    .from("flow_deployments")
    .update({
      output_instructions: true,
      start_permission: "Anonymous",
      fees: [[feeRecipient.toBase58(), feeAmount]],
    })
    .eq("id", id);
  if (updateResult.error) throw new Error(JSON.stringify(updateResult.error));

  const starterKeypair = web3.Keypair.generate();
  const starter = new client.Client({
    host: "http://localhost:8080",
    anonKey,
  });
  starter.setToken(starterKeypair.publicKey.toString());
  const { flow_run_id, token } = await starter.startDeployment(
    {
      flow: flowId,
      tag: "latest",
    },
    {
      inputs: new Value({
        sender: starterKeypair.publicKey,
      }).M!,
    },
  );
  starter.setToken(token);
  const output = await starter.getFlowOutput(flow_run_id);
  const text = output.toJSObject().transaction;
  await checkNoErrors(sup, flow_run_id);
  const tx = web3.VersionedTransaction.deserialize(decodeBase64(text));
  const msg = web3.TransactionMessage.decompile(tx.message);
  const transfer = web3.SystemInstruction.decodeTransfer(msg.instructions[2]);
  assertEquals(transfer.fromPubkey, starterKeypair.publicKey);
  const fee = web3.SystemInstruction.decodeTransfer(msg.instructions[3]);
  assertEquals(fee.fromPubkey, starterKeypair.publicKey);
  assertEquals(fee.toPubkey, feeRecipient);
  assertEquals(fee.lamports, BigInt(feeAmount));
});

Deno.test("action identity", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });
  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);

  const flowId = DEPLOY_ACTION_FLOW_ID;
  const id = await owner.deployFlow(flowId);
  const identity = web3.Keypair.generate();
  owner.setToken(`Bearer ${jwt.access_token}`);
  await owner.upsertWallet({
    type: "HARDCODED",
    name: "identity",
    public_key: identity.publicKey.toBase58(),
    keypair: bs58.encodeBase58(identity.secretKey),
    user_id: (await sup.auth.getSession()).data.session?.user.id,
  });
  const updateResult = await sup
    .from("flow_deployments")
    .update({
      output_instructions: true,
      start_permission: "Anonymous",
      action_identity: identity.publicKey.toBase58(),
    })
    .eq("id", id);
  if (updateResult.error) throw new Error(JSON.stringify(updateResult.error));

  const starterKeypair = web3.Keypair.generate();
  const starter = new client.Client({
    host: "http://localhost:8080",
    anonKey,
  });
  starter.setToken(starterKeypair.publicKey.toString());
  const { flow_run_id, token } = await starter.startDeployment(
    {
      flow: flowId,
      tag: "latest",
    },
    {
      inputs: new Value({
        sender: starterKeypair.publicKey,
      }).M!,
    },
  );
  starter.setToken(token);
  const output = await starter.getFlowOutput(flow_run_id);
  const text = output.toJSObject().transaction;
  await checkNoErrors(sup, flow_run_id);
  const tx = web3.VersionedTransaction.deserialize(decodeBase64(text));
  const msg = web3.TransactionMessage.decompile(tx.message);
  const transfer = web3.SystemInstruction.decodeTransfer(msg.instructions[2]);
  assertEquals(transfer.fromPubkey, starterKeypair.publicKey);
  const memo = msg.instructions[3].data.toString("utf-8");
  assert(memo.startsWith(`solana-action:${identity.publicKey.toBase58()}`));
});

Deno.test("execute on action", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });
  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);
  owner.setToken(`Bearer ${jwt.access_token}`);
  const user_id = (await sup.auth.getSession()).data.session?.user.id;

  const flowId = DEPLOY_ACTION_FLOW_ID;
  const id = await owner.deployFlow(flowId);
  const identity = web3.Keypair.generate();
  await owner.upsertWallet({
    type: "HARDCODED",
    name: "identity",
    public_key: identity.publicKey.toBase58(),
    keypair: bs58.encodeBase58(identity.secretKey),
    user_id,
  });
  const updateResult = await sup
    .from("flow_deployments")
    .update({
      start_permission: "Anonymous",
      action_identity: identity.publicKey.toBase58(),
    })
    .eq("id", id);
  if (updateResult.error) throw new Error(JSON.stringify(updateResult.error));

  const starterKeypair = web3.Keypair.generate();
  const starter = new client.Client({
    host: "http://localhost:8080",
    anonKey,
  });
  starter.setToken(starterKeypair.publicKey.toString());

  const conn = new web3.Connection(getEnv("SOLANA_DEVNET_URL"));
  const ownerKeypair = web3.Keypair.fromSecretKey(
    bs58.decodeBase58(getEnv("KEYPAIR")),
  );
  const recent = await conn.getLatestBlockhash();
  const fundTx = web3.Transaction.populate(
    web3.Message.compile({
      instructions: [
        web3.SystemProgram.transfer({
          fromPubkey: ownerKeypair.publicKey,
          toPubkey: starterKeypair.publicKey,
          lamports: 0.1 * LAMPORTS_PER_SOL + 10000,
        }),
      ],
      payerKey: ownerKeypair.publicKey,
      recentBlockhash: recent.blockhash,
    }),
  );
  const fundSignature = await web3.sendAndConfirmTransaction(conn, fundTx, [
    ownerKeypair,
  ]);
  console.log("fundSignature", fundSignature);

  const { flow_run_id, token } = await starter.startDeployment(
    {
      flow: flowId,
      tag: "latest",
    },
    {
      inputs: new Value({
        sender: starterKeypair.publicKey,
      }).M!,
      action_signer: starterKeypair.publicKey.toBase58(),
    },
  );
  starter.setToken(token);

  const req = await starter.getSignatureRequest(flow_run_id);
  const tx = req.buildTransaction();
  tx.sign([starterKeypair]);
  console.log(tx.signatures.map((sig) => encodeBase58(sig)));

  const signature = await conn.sendTransaction(tx, {
    skipPreflight: true,
  });

  const output = (await starter.getFlowOutput(flow_run_id)).toJSObject();
  await checkNoErrors(sup, flow_run_id);
  assertEquals(bs58.decodeBase58(signature), output.signature);
});

Deno.test("start authenticated by flow", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });

  const flowId = DEPLOY_SIMPLE_FLOW_ID;
  const id = await owner.deployFlow(flowId);
  const ownerSup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await ownerSup.auth.setSession(await owner.claimToken());
  const updateResult = await ownerSup
    .from("flow_deployments")
    .update({
      start_permission: "Authenticated",
    })
    .eq("id", id);
  if (updateResult.error) throw new Error(JSON.stringify(updateResult.error));

  const starter = new client.Client({
    host: "http://localhost:8080",
    anonKey,
  });
  const starterKeypair = web3.Keypair.generate();
  const initAuth = await starter.initAuth(starterKeypair.publicKey);
  const signature = ed25519SignText(starterKeypair, initAuth);
  const { session } = await starter.confirmAuth(initAuth, signature);
  starter.setToken(`Bearer ${session.access_token}`);

  const { flow_run_id } = await starter.startDeployment(
    {
      flow: flowId,
    },
    {
      inputs: new Value({
        a: 1,
        b: 2,
      }).M!,
    },
  );

  const result = await starter.getFlowOutput(flow_run_id);
  const c = result.toJSObject().c;
  assertEquals(c, 3);

  const starterSup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await starterSup.auth.setSession(session);
  await checkNoErrors(starterSup, flow_run_id);
  const selectResult = await starterSup
    .from("flow_run")
    .select("output")
    .eq("deployment_id", id)
    .single();
  if (selectResult.error) throw new Error(JSON.stringify(selectResult.error));
  assertEquals(
    Value.fromJSON(selectResult.data.output as client.IValue),
    result,
  );
});

Deno.test("start by flow + tag", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });

  const flowId = DEPLOY_SIMPLE_FLOW_ID;
  await owner.deployFlow(flowId);

  const { flow_run_id } = await owner.startDeployment(
    {
      flow: flowId,
      tag: "latest",
    },
    {
      inputs: new Value({
        a: 1,
        b: 2,
      }).M!,
    },
  );

  const result = await owner.getFlowOutput(flow_run_id);
  const c = result.toJSObject().c;
  assertEquals(c, 3);

  const jwt = await owner.claimToken();
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  await sup.auth.setSession(jwt);
  await checkNoErrors(sup, flow_run_id);
});

Deno.test("start custom tag", async () => {
  const owner = new client.Client({
    host: "http://localhost:8080",
    anonKey,
    token: apiKey,
  });
  const sup = createClient<client.Database>(supabaseUrl, anonKey, {
    auth: { autoRefreshToken: false },
  });
  const jwt = await owner.claimToken();
  await sup.auth.setSession(jwt);
  const user_id = (await sup.auth.getUser()).data.user?.id!;

  const flowId = DEPLOY_SIMPLE_FLOW_ID;
  const id = await owner.deployFlow(flowId);
  await sup.from("flow_deployments_tags").upsert({
    deployment_id: id,
    entrypoint: flowId,
    tag: "v1",
    user_id,
  });

  const { flow_run_id } = await owner.startDeployment(
    {
      flow: flowId,
      tag: "v1",
    },
    {
      inputs: new Value({
        a: 1,
        b: 2,
      }).M!,
    },
  );

  const result = await owner.getFlowOutput(flow_run_id);
  const c = result.toJSObject().c;
  assertEquals(c, 3);

  await checkNoErrors(sup, flow_run_id);
});
