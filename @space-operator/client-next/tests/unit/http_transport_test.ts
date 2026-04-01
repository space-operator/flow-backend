import { assertEquals, assertRejects } from "@std/assert";
import { createClient, TimeoutError } from "../../src/mod.ts";
import { requestJsonWithMeta } from "../../src/internal/transport/http.ts";
import { resolveClientConfig } from "../../src/internal/runtime.ts";

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

unitTest(
  "http transport treats 304 as a non-error metadata response",
  async () => {
    let attempts = 0;
    const response = await requestJsonWithMeta(
      resolveClientConfig({
        baseUrl: "http://example.test",
        retry: {
          attempts: 3,
          backoffMs: 0,
          retryableStatusCodes: [304, 500],
        },
        fetch: async () => {
          attempts += 1;
          return new Response(null, {
            status: 304,
            headers: {
              ETag: '"abc"',
              "Cache-Control": "public, max-age=60",
              "Last-Modified": "Tue, 31 Mar 2026 12:00:00 GMT",
            },
          });
        },
      }),
      {
        method: "GET",
        path: "/flow/read/flow-1",
      },
    );

    assertEquals(attempts, 1);
    assertEquals(response.status, 304);
    assertEquals(response.body, undefined);
    assertEquals(response.etag, '"abc"');
    assertEquals(response.cacheControl, "public, max-age=60");
    assertEquals(response.lastModified, "Tue, 31 Mar 2026 12:00:00 GMT");
  },
);
