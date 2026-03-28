import { assert, assertEquals } from "@std/assert";
import { contractTest, FLOW_SERVER_URL } from "./_shared.ts";
import { createClient } from "../../src/mod.ts";

contractTest(
  "service contract: info exposes server configuration",
  async () => {
    const client = createClient({ baseUrl: FLOW_SERVER_URL });
    const info = await client.service.info();

    assert(info.anon_key.length > 0);
    assert(new URL(info.supabase_url).protocol.length > 0);
    assertEquals(new URL(info.base_url).host, new URL(FLOW_SERVER_URL).host);
    assert(info.iroh.node_id.length > 0);
    assert(info.iroh.relay_url.length > 0);
  },
);

contractTest(
  "service contract: healthcheck still reports success",
  async () => {
    const client = createClient({ baseUrl: FLOW_SERVER_URL });
    const result = await client.service.healthcheck();

    assertEquals(result, { success: true });
  },
);
