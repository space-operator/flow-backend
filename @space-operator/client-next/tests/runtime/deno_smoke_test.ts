import { assertEquals } from "@std/assert";
import { createClient } from "../../src/mod.ts";

Deno.test("deno smoke: service namespace works with injected fetch", async () => {
  const client = createClient({
    baseUrl: "http://example.test",
    fetch: async () => Response.json({ success: true }),
  });

  assertEquals(await client.service.healthcheck(), { success: true });
});
