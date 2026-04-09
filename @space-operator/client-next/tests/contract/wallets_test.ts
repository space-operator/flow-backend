import { assert, assertEquals } from "@std/assert";
import { encodeBase58 } from "../../src/deps.ts";
import {
  contractTest,
  ownerBearerClient,
  randomName,
  supabase,
  web3,
} from "./_shared.ts";

contractTest(
  "wallets contract: upsert persists a wallet row",
  async () => {
    const { client, session } = await ownerBearerClient();
    const db = supabase();
    await db.auth.setSession(session as never);

    const keypair = web3.Keypair.generate();
    const name = randomName("e2e-wallet");
    const publicKey = keypair.publicKey.toBase58();

    try {
      const result = await client.wallets.upsert<
        Array<{
          id: number;
          name: string;
          public_key: string;
          user_id: string;
        }>
      >({
        type: "HARDCODED",
        name,
        public_key: publicKey,
        keypair: encodeBase58(keypair.secretKey),
        user_id: session.user_id,
      });

      assert(Array.isArray(result));
      assert(result.length > 0);
      assertEquals(result[0].name, name);
      assertEquals(result[0].public_key, publicKey);
      assertEquals(result[0].user_id, session.user_id);

      const row = await db
        .from("wallets")
        .select("name, public_key, user_id")
        .eq("name", name)
        .eq("public_key", publicKey)
        .single();
      if (row.error) {
        throw new Error(JSON.stringify(row.error));
      }

      assertEquals(row.data.name, name);
      assertEquals(row.data.public_key, publicKey);
      assertEquals(row.data.user_id, session.user_id);
    } finally {
      await db
        .from("wallets")
        .delete()
        .eq("name", name)
        .eq("public_key", publicKey);
    }
  },
);
