import { assertEquals, assertRejects } from "@std/assert";
import { createClient, TimeoutError } from "../../src/mod.ts";

function readSignal(init: unknown): AbortSignal | null | undefined {
  return (init as { signal?: AbortSignal | null } | undefined)?.signal;
}

function unitTest(name: string, fn: () => Promise<void>) {
  Deno.test({
    name,
    sanitizeOps: false,
    sanitizeResources: false,
    fn,
  });
}

unitTest(
  "http transport retries transient fetch failures before succeeding",
  async () => {
    let attempts = 0;
    const client = createClient({
      baseUrl: "http://example.test",
      retry: {
        attempts: 2,
        backoffMs: 0,
      },
      fetch: async () => {
        attempts += 1;
        if (attempts === 1) {
          throw new Error("temporary network failure");
        }
        return Response.json({ success: true });
      },
    });

    const response = await client.service.healthcheck();

    assertEquals(response, { success: true });
    assertEquals(attempts, 2);
  },
);

unitTest("http transport still surfaces timeout errors", async () => {
  const client = createClient({
    baseUrl: "http://example.test",
    timeoutMs: 5,
    fetch: async (_input, init) =>
      await new Promise<Response>((_resolve, reject) => {
        readSignal(init)?.addEventListener(
          "abort",
          () => reject(new DOMException("aborted", "AbortError")),
          { once: true },
        );
      }),
  });

  await assertRejects(
    () => client.service.info(),
    TimeoutError,
    "timed out",
  );
});
