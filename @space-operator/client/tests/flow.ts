import * as client from "../src/mod.ts";
import { Value, web3, bs58 } from "../src/deps.ts";

const c = new client.Client({});

const keypair = web3.Keypair.generate();
const connection = new web3.Connection("https://api.devnet.solana.com");
await connection.requestAirdrop(keypair.publicKey, web3.LAMPORTS_PER_SOL);
while ((await connection.getBalance(keypair.publicKey)) == 0) {}
const result = await c.startFlowUnverified(2154, keypair.publicKey, {
  inputs: new Value({
    sender: keypair.publicKey,
  }).M!,
});

const req = await c.getSignatureRequest(result.flow_run_id, result.token);

const tx = req.buildTransaction();

tx.sign(keypair);

const sigResult = await c.submitSignature({
  id: req.id,
  signature: bs58.encodeBase58(tx.signature!),
});

console.log(sigResult);

const output = await c.getFlowOutput(result.flow_run_id, result.token);

console.log(output);
