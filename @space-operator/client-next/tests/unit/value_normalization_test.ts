import { assertEquals, assertExists } from "@std/assert";
import { createClient } from "../../src/mod.ts";

Deno.test("normalizes plain js objects and IValue inputs into flow values", async () => {
  let requestBody: Record<string, unknown> | undefined;
  const client = createClient({
    baseUrl: "http://example.test",
    fetch: async (_input, init) => {
      requestBody = JSON.parse(String(init?.body));
      return Response.json({ flow_run_id: "run-1" });
    },
  });

  await client.flows.start("flow-1", {
    inputs: {
      plain: { count: 1 },
      typed: { S: "hello" },
    },
  });

  const inputs = requestBody?.inputs as Record<string, unknown>;
  assertExists(inputs);
  assertEquals(inputs.typed, { S: "hello" });
  assertEquals(inputs.plain, {
    M: {
      count: { D: "1" },
    },
  });
});
