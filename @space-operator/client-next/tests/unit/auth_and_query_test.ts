import { assertEquals } from "@std/assert";
import { apiKeyAuth, createClient, publicKeyAuth } from "../../src/mod.ts";

function readInit(
  init: unknown,
): { headers: Headers; body: BodyInit | null | undefined } {
  const value = init as {
    headers?: HeadersInit;
    body?: BodyInit | null;
  } | undefined;
  return {
    headers: new Headers(value?.headers),
    body: value?.body,
  };
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
  "uses explicit api key auth and preserves base path prefixes",
  async () => {
    const requests: Array<{ url: string; headers: Headers; body?: unknown }> =
      [];
    const client = createClient({
      baseUrl: "http://example.test/api/v1",
      auth: apiKeyAuth("b3-demo"),
      fetch: async (input, init) => {
        const request = readInit(init);
        requests.push({
          url: String(input),
          headers: request.headers,
          body: request.body,
        });
        return Response.json({
          user_id: "user-1",
          access_token: "jwt",
          refresh_token: "refresh",
          expires_at: 123,
        });
      },
    });

    const token = await client.auth.claimToken();

    assertEquals(token.user_id, "user-1");
    assertEquals(
      requests[0].url,
      "http://example.test/api/v1/auth/claim_token",
    );
    assertEquals(requests[0].headers.get("x-api-key"), "b3-demo");
  },
);

unitTest("uses explicit public key auth for unverified starts", async () => {
  const requests: Array<{ headers: Headers; body?: unknown }> = [];
  const client = createClient({
    baseUrl: "http://example.test",
    auth: publicKeyAuth("PubKey1111111111111111111111111111111111"),
    fetch: async (_input, init) => {
      const request = readInit(init);
      requests.push({
        headers: request.headers,
        body: request.body,
      });
      return Response.json({
        flow_run_id: "run-1",
        token: "frt-demo",
      });
    },
  });

  const run = await client.flows.startUnverified("flow-1", {
    inputs: {
      greeting: "hello",
    },
  });

  assertEquals(run.id, "run-1");
  assertEquals(
    requests[0].headers.get("authorization"),
    "Bearer PubKey1111111111111111111111111111111111",
  );
});

unitTest("signature auth uses the configured anon key header", async () => {
  const requests: Array<{ url: string; headers: Headers }> = [];
  const client = createClient({
    baseUrl: "http://example.test",
    anonKey: "anon-123",
    fetch: async (input, init) => {
      const request = readInit(init);
      requests.push({
        url: String(input),
        headers: request.headers,
      });
      return Response.json({ msg: "sign-me" });
    },
  });

  const message = await client.auth.init(
    "PubKey1111111111111111111111111111111111",
  );

  assertEquals(message, "sign-me");
  assertEquals(requests[0].url, "http://example.test/auth/init");
  assertEquals(requests[0].headers.get("apikey"), "anon-123");
});

unitTest(
  "loginWithSignature can discover and reuse the anon key from service info",
  async () => {
    const requests: Array<{ url: string; headers: Headers; body?: unknown }> =
      [];
    const client = createClient({
      baseUrl: "http://example.test",
      fetch: async (input, init) => {
        const url = String(input);
        const request = readInit(init);
        requests.push({
          url,
          headers: request.headers,
          body: request.body,
        });

        if (url.endsWith("/info")) {
          return Response.json({
            supabase_url: "http://supabase.example.test",
            anon_key: "anon-from-info",
            iroh: {
              node_id: "node-1",
              relay_url: "https://relay.example.test",
              direct_addresses: [],
            },
            base_url: "http://example.test",
          });
        }
        if (url.endsWith("/auth/init")) {
          return Response.json({ msg: "sign-this-message" });
        }
        if (url.endsWith("/auth/confirm")) {
          return Response.json({
            session: {
              access_token: "jwt",
              refresh_token: "refresh",
              token_type: "bearer",
              expires_in: 3600,
              expires_at: 123,
              user: {
                id: "user-1",
                aud: "authenticated",
                created_at: "2026-03-26T00:00:00Z",
                app_metadata: {},
                user_metadata: {},
              },
            },
            new_user: false,
          });
        }
        throw new Error(`unexpected request ${url}`);
      },
    });

    const result = await client.auth.loginWithSignature(
      "PubKey1111111111111111111111111111111111",
      async (message) => {
        assertEquals(message, "sign-this-message");
        return "signed-message";
      },
    );

    assertEquals(result.new_user, false);
    assertEquals(requests.map((request) => request.url), [
      "http://example.test/info",
      "http://example.test/auth/init",
      "http://example.test/auth/confirm",
    ]);
    assertEquals(requests[1].headers.get("apikey"), "anon-from-info");
    assertEquals(requests[2].headers.get("apikey"), "anon-from-info");
  },
);

unitTest("deployment reads omit undefined inputs from GET queries", async () => {
  const requests: string[] = [];
  const client = createClient({
    baseUrl: "http://example.test",
    fetch: async (input) => {
      requests.push(String(input));
      return new Response(JSON.stringify({ N: 0 }), {
        status: 200,
        headers: {
          "Content-Type": "application/json",
          "Cache-Control": "private, max-age=60",
          ETag: '"etag-1"',
        },
      });
    },
  });

  const result = await client.deployments.read({ id: "dep-1" });

  assertEquals(result.value.toJSObject(), null);
  assertEquals(requests, ["http://example.test/deployment/read?id=dep-1"]);
});
