import { assertEquals, assertRejects } from "@std/assert";
import { apiKeyAuth, createClient } from "../../src/mod.ts";
import {
  contractTest,
  FLOW_SERVER_URL,
  ownerBearerClient,
  randomName,
} from "./_shared.ts";

contractTest(
  "api keys contract: create, inspect, and delete a key",
  async () => {
    const { client: owner, session } = await ownerBearerClient();
    const name = randomName("e2e-apikey");
    let keyHash: string | undefined;
    let fullKey: string | undefined;

    try {
      const created = await owner.apiKeys.create(name);
      keyHash = created.key_hash;
      fullKey = created.full_key;

      assertEquals(created.name, name);
      assertEquals(created.user_id, session.user_id);

      const apiKeyClient = createClient({
        baseUrl: FLOW_SERVER_URL,
        auth: apiKeyAuth(created.full_key),
      });
      const info = await apiKeyClient.apiKeys.info();

      assertEquals(info.user_id, session.user_id);
    } finally {
      if (keyHash) {
        await owner.apiKeys.delete(keyHash);
      }
    }

    if (fullKey) {
      const deletedClient = createClient({
        baseUrl: FLOW_SERVER_URL,
        auth: apiKeyAuth(fullKey),
      });
      await assertRejects(() => deletedClient.apiKeys.info());
    }
  },
);
