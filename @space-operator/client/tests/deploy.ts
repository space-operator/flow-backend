import { bs58, Value, web3 } from "../src/deps.ts";
import * as client from "../src/mod.ts";
import * as dotenv from "jsr:@std/dotenv";

dotenv.loadSync({
  export: true,
});

const anonKey = Deno.env.get("ANON_KEY");
if (!anonKey) throw new Error("no ANON_KEY");

const token = Deno.env.get("APIKEY");
if (!token) throw new Error("no APIKEY");

const owner = new client.Client({
  host: "http://localhost:8080",
  supabaseUrl: "http://localhost:8000",
  anonKey,
  token,
});
const ownerKeypair = web3.Keypair.fromSecretKey(
  bs58.decodeBase58(Deno.env.get("KEYPAIR") ?? "")
);

const run = async () => {
  const flowId = 3643;
  console.log("deploy flow", flowId);
  const id = await owner.deployFlow(flowId);

  const keypair = web3.Keypair.generate();

  const starter = new client.Client({
    host: "http://localhost:8080",
    supabaseUrl: "http://localhost:8000",
    anonKey,
  });

  starter.setToken(keypair.publicKey.toString());
  console.log("start deployment", id);
  const { flow_run_id, token } = await starter.startDeployment(
    {
      id,
    },
    {
      inputs: new Value({
        sender: keypair.publicKey,
        n: 2,
      }).M!,
    }
  );
  console.log("flow_run_id", flow_run_id);

  {
    console.log("getSignatureRequest");
    const req = await owner.getSignatureRequest(flow_run_id);
    console.log("req id", req.id);
    await owner.signAndSubmitSignature(
      req,
      ownerKeypair.publicKey,
      async (tx) => {
        tx.sign([ownerKeypair]);
        return tx;
      }
    );
  }

  {
    console.log("getSignatureRequest");
    const req = await starter.getSignatureRequest(flow_run_id, token);
    console.log("req id", req.id);
    await starter.signAndSubmitSignature(req, keypair.publicKey, async (tx) => {
      tx.sign([keypair]);
      return tx;
    });
  }

  const result = await starter.getFlowOutput(flow_run_id, token);
  return result;
};

const res = await Promise.all([
  (async () => {
    try {
      await run();
    } catch (error) {
      const sup = await owner.supabase();
      const result = await sup
        .from("node_run")
        .select("errors")
        .not("errors", "is", "null");
      console.log(result);

      throw error;
    }
  })(),
]);

console.log(res);
