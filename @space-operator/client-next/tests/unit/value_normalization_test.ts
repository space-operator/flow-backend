import { assertEquals, assertExists } from "@std/assert";
import { Value } from "@space-operator/flow-lib";
import { Keypair } from "@solana/web3.js";
import { createClient } from "../../src/mod.ts";

function readBody(init: unknown): BodyInit | null | undefined {
  return (init as { body?: BodyInit | null } | undefined)?.body;
}

Deno.test("normalizes plain js objects and IValue inputs into flow values", async () => {
  let requestBody: Record<string, unknown> | undefined;
  const client = createClient({
    baseUrl: "http://example.test",
    fetch: async (_input, init) => {
      requestBody = JSON.parse(String(readBody(init)));
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

Deno.test("normalizes keypair-shaped inputs into stable B6 values", async () => {
  let requestBody: Record<string, unknown> | undefined;
  const client = createClient({
    baseUrl: "http://example.test",
    fetch: async (_input, init) => {
      requestBody = JSON.parse(String(readBody(init)));
      return Response.json({ flow_run_id: "run-1" });
    },
  });

  const keypair = Keypair.generate();
  const expected = Value.Keypair(keypair.secretKey).B6;

  await client.flows.start("flow-1", {
    inputs: {
      typed: Value.Keypair(keypair.secretKey),
      raw_keypair: keypair,
      raw_bytes: keypair.secretKey,
      explicit_b6: { B6: expected! },
    },
  });

  const inputs = requestBody?.inputs as Record<string, { B6?: string }>;
  assertExists(inputs);
  assertEquals(inputs.typed.B6, expected);
  assertEquals(inputs.raw_keypair.B6, expected);
  assertEquals(inputs.raw_bytes.B6, expected);
  assertEquals(inputs.explicit_b6.B6, expected);
});
