import { ApiError, createClient, publicKeyAuth } from "../../src/mod.ts";
import { assert, assertEquals, assertRejects } from "@std/assert";
import {
  apiClient,
  contractTest,
  FLOW_SERVER_URL,
  getEnv,
  resolveFixtureFlowId,
  SUPABASE_URL,
  web3,
} from "./_shared.ts";

type FlowV2Flags = {
  start_shared?: boolean;
  start_unverified?: boolean;
  is_public?: boolean;
  read_enabled?: boolean;
};

type FlowRunRow = {
  id: string;
  origin: Record<string, unknown>;
  start_time: string | null;
  end_time: string | null;
};

async function requestFlowV2<T>(
  method: "GET" | "PATCH" | "DELETE",
  query: string,
  body?: unknown,
): Promise<T> {
  const serviceRoleKey = getEnv("SERVICE_ROLE_KEY");
  const response = await fetch(`${SUPABASE_URL}/rest/v1/flows_v2${query}`, {
    method,
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
      ...(body ? { "content-type": "application/json" } : {}),
      ...(method === "PATCH" ? { prefer: "return=representation" } : {}),
    },
    ...(body ? { body: JSON.stringify(body) } : {}),
  });
  const text = await response.text();
  if (!response.ok) {
    throw new Error(
      `flows_v2 ${method} failed: ${response.status} ${text || response.statusText}`,
    );
  }
  return text.length === 0 ? [] as T : JSON.parse(text) as T;
}

async function waitForReadRunCompletion(
  flowId: string,
  startedAt: Date,
): Promise<FlowRunRow> {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    const runs = await requestFlowRuns<FlowRunRow[]>(
      `?flow_id=eq.${flowId}&start_time=gte.${
        encodeURIComponent(startedAt.toISOString())
      }&select=id,origin,start_time,end_time&order=start_time.asc`,
    );
    const readRun = runs.find((row) =>
      row.origin != null && Object.hasOwn(row.origin, "Read")
    );
    if (readRun?.end_time != null) {
      return readRun;
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }

  throw new Error(`timed out waiting for read run completion for ${flowId}`);
}

async function deleteFlowV2Rows(ids: string[]): Promise<void> {
  if (ids.length === 0) {
    return;
  }
  const serviceRoleKey = getEnv("SERVICE_ROLE_KEY");
  const query =
    `?uuid=in.(${ids.map((id) => `"${id}"`).join(",")})`;
  const response = await fetch(`${SUPABASE_URL}/rest/v1/flows_v2${query}`, {
    method: "DELETE",
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
      prefer: "return=minimal",
    },
  });
  const text = await response.text();
  if (!response.ok) {
    throw new Error(
      `flows_v2 DELETE failed: ${response.status} ${text || response.statusText}`,
    );
  }
}

async function requestFlowRuns<T>(query: string): Promise<T> {
  const serviceRoleKey = getEnv("SERVICE_ROLE_KEY");
  const response = await fetch(`${SUPABASE_URL}/rest/v1/flow_run${query}`, {
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
    },
  });
  const text = await response.text();
  if (!response.ok) {
    throw new Error(
      `flow_run GET failed: ${response.status} ${text || response.statusText}`,
    );
  }
  return text.length === 0 ? [] as T : JSON.parse(text) as T;
}

async function withFlowFlags(
  flowId: string,
  flags: {
    start_shared?: boolean;
    start_unverified?: boolean;
    isPublic?: boolean;
    read_enabled?: boolean;
  },
  fn: () => Promise<void>,
) {
  const query =
    `?uuid=eq.${flowId}&select=uuid,start_shared,start_unverified,is_public,read_enabled`;
  const [original] = await requestFlowV2<Array<{
    uuid: string;
    start_shared: boolean;
    start_unverified: boolean;
    is_public: boolean;
    read_enabled: boolean;
  }>>("GET", query);
  if (!original) {
    throw new Error(`missing flows_v2 row for ${flowId}`);
  }

  const [updated] = await requestFlowV2<Array<{
    start_shared: boolean;
    start_unverified: boolean;
    is_public: boolean;
    read_enabled: boolean;
  }>>("PATCH", query, {
    start_shared: flags.start_shared,
    start_unverified: flags.start_unverified,
    is_public: flags.isPublic,
    read_enabled: flags.read_enabled,
  });
  for (const [key, value] of Object.entries({
    start_shared: flags.start_shared,
    start_unverified: flags.start_unverified,
    is_public: flags.isPublic,
    read_enabled: flags.read_enabled,
  })) {
    if (value !== undefined && (updated as Record<string, unknown>)[key] !== value) {
      throw new Error(
        `failed to set fixture flow flag ${key}=${value} for ${flowId}`,
      );
    }
  }

  try {
    await fn();
  } finally {
    await requestFlowV2("PATCH", query, {
      start_shared: original.start_shared,
      start_unverified: original.start_unverified,
      is_public: original.is_public,
      read_enabled: original.read_enabled,
    });
  }
}

contractTest("flow contract: start and fetch output", async () => {
  const client = apiClient();
  const startFlowId = await resolveFixtureFlowId("start");
  const run = await client.flows.start(startFlowId, {
    inputs: {
      a: 1,
      b: 2,
    },
  });

  const output = await run.output();
  assertEquals(output.toJSObject().c, 3);
});

contractTest("flow contract: deno node output is preserved", async () => {
  const client = apiClient();
  const denoFlowId = await resolveFixtureFlowId("deno");
  const run = await client.flows.start(denoFlowId);
  const output = await run.output();

  assertEquals(output.toJSObject().pi, 3.14);
});

contractTest(
  "flow contract: interflow still resolves nested runs",
  async () => {
    const client = apiClient();
    const interflowFlowId = await resolveFixtureFlowId("interflow");
    const run = await client.flows.start(interflowFlowId, {
      inputs: {
        n: 54,
      },
    });
    const output = await run.output();
    const value = output.toJSObject();

    assertEquals(value.count, 112);
    assertEquals(value.out, 1);
  },
);

contractTest(
  "flow contract: interflow instructions remain available",
  async () => {
    const client = apiClient();
    const interflowInstructionsFlowId = await resolveFixtureFlowId(
      "interflowInstructions",
    );
    const run = await client.flows.start(interflowInstructionsFlowId);
    const output = await run.output();

    assert(output.toJSObject().ins != null);
  },
);

contractTest("flow contract: const flows still complete", async () => {
  const client = apiClient();
  const constsFlowId = await resolveFixtureFlowId("consts");
  const run = await client.flows.start(constsFlowId);

  await run.output();
});

contractTest(
  "flow contract: clone still duplicates runnable flows",
  async () => {
    const client = apiClient();
    let cloneIds: string[] = [];

    try {
      const startFlowId = await resolveFixtureFlowId("start");
      const cloned = await client.flows.clone(startFlowId);
      cloneIds = Object.values(cloned.id_map);

      assert(cloned.flow_id !== startFlowId);
      assertEquals(cloned.id_map[startFlowId], cloned.flow_id);

      const run = await client.flows.start(cloned.flow_id, {
        inputs: {
          a: 4,
          b: 5,
        },
      });
      const output = await run.output();

      assertEquals(output.toJSObject().c, 9);
    } finally {
      if (cloneIds.length > 0) {
        await deleteFlowV2Rows(cloneIds);
      }
    }
  },
);

contractTest(
  "flow contract: read requires read_enabled and returns snapshot output",
  async () => {
    const startFlowId = await resolveFixtureFlowId("start");
    const client = apiClient();

    const error = await assertRejects(
      () =>
        client.flows.read(startFlowId, {
          inputs: {
            a: 2,
            b: 3,
          },
          skipCache: true,
        }),
      ApiError,
    );
    assertEquals(error.status, 403);

    await withFlowFlags(
      startFlowId,
      { read_enabled: true },
      async () => {
        const result = await client.flows.read(startFlowId, {
          inputs: {
            a: 2,
            b: 3,
          },
          skipCache: true,
        });

        assertEquals(result.value.toJSObject().c, 5);
      },
    );
  },
);

contractTest(
  "flow contract: repeated reads stay under budget and collapse to one read run",
  async () => {
    const startFlowId = await resolveFixtureFlowId("start");

    await withFlowFlags(
      startFlowId,
      { read_enabled: true },
      async () => {
        const startedAt = new Date();
        const perRequestMs: number[] = [];

        const wallStart = performance.now();
        for (let i = 0; i < 10; i++) {
          const requestStart = performance.now();
          const result = await apiClient().flows.read(startFlowId, {
            inputs: {
              a: 2,
              b: 3,
            },
          });
          perRequestMs.push(performance.now() - requestStart);
          assertEquals(result.value.toJSObject().c, 5);
        }
        const elapsedMs = performance.now() - wallStart;

        const runs = await requestFlowRuns<FlowRunRow[]>(
          `?flow_id=eq.${startFlowId}&start_time=gte.${
            encodeURIComponent(startedAt.toISOString())
          }&select=id,origin,start_time,end_time&order=start_time.asc`,
        );
        const readRuns = runs.filter((row) =>
          row.origin != null && Object.hasOwn(row.origin, "Read")
        );

        assert(
          elapsedMs <= 2_000,
          `expected 10 reads in <= 2000ms, got ${elapsedMs.toFixed(2)}ms`,
        );
        assertEquals(
          readRuns.length,
          1,
          `expected one persisted read run, got ${readRuns.length}; per-request timings: ${
            perRequestMs.map((ms) => ms.toFixed(2)).join(", ")
          }`,
        );
        assert(readRuns[0].end_time != null, "expected cached read run to finish");
      },
    );
  },
);

contractTest(
  "flow contract: read returns ETag headers and honors If-None-Match",
  async () => {
    const startFlowId = await resolveFixtureFlowId("start");

    await withFlowFlags(
      startFlowId,
      { read_enabled: true },
      async () => {
        const inputs = encodeURIComponent(JSON.stringify({
          a: { D: "2" },
          b: { D: "3" },
        }));
        const url = `${FLOW_SERVER_URL}/flow/read/${startFlowId}?inputs=${inputs}`;
        const headers = { "x-api-key": getEnv("APIKEY") };

        const first = await fetch(url, { headers });
        const firstBody = await first.json();
        const etag = first.headers.get("etag");
        const cacheControl = first.headers.get("cache-control");
        const lastModified = first.headers.get("last-modified");

        assertEquals(first.status, 200);
        assertEquals(firstBody, {
          M: {
            c: { D: "5" },
          },
        });
        assert(etag != null && etag.length > 0, "expected ETag header");
        assert(
          cacheControl != null && cacheControl.length > 0,
          "expected Cache-Control header",
        );
        assert(
          lastModified != null && lastModified.length > 0,
          "expected Last-Modified header",
        );

        const second = await fetch(url, {
          headers: {
            ...headers,
            "if-none-match": etag,
          },
        });
        const secondBody = await second.text();

        assertEquals(second.status, 304);
        assertEquals(secondBody, "");
      },
    );
  },
);

contractTest(
  "flow contract: timed out reads force-stop and persist end_time",
  async () => {
    const client = apiClient();
    const denoFlowId = await resolveFixtureFlowId("deno");
    let cloneIds: string[] = [];

    try {
      const cloned = await client.flows.clone(denoFlowId);
      cloneIds = Object.values(cloned.id_map);

      const [cloneRow] = await requestFlowV2<Array<{
        uuid: string;
        nodes: Array<Record<string, unknown>>;
      }>>("GET", `?uuid=eq.${cloned.flow_id}&select=uuid,nodes`);
      if (!cloneRow) {
        throw new Error(`missing cloned flow row for ${cloned.flow_id}`);
      }

      const nodes = structuredClone(cloneRow.nodes);
      const denoNode = nodes.find((node) => node.type === "deno");
      if (!denoNode) {
        throw new Error("missing deno node in cloned flow");
      }

      const data = denoNode.data as {
        config?: { source?: string };
      } | undefined;
      if (!data?.config) {
        throw new Error("missing deno node config in cloned flow");
      }
      data.config.source = [
        'import { BaseCommand, Context } from "jsr:@space-operator/flow-lib@0.15.0";',
        "",
        "export default class MyCommand extends BaseCommand {",
        "  override async run(_: Context, inputs: any): Promise<any> {",
        "    await new Promise((resolve) => setTimeout(resolve, 35_000));",
        "    return { c: inputs.a + inputs.b };",
        "  }",
        "}",
      ].join("\n");

      await requestFlowV2(
        "PATCH",
        `?uuid=eq.${cloned.flow_id}&select=uuid,read_enabled,nodes`,
        {
          read_enabled: true,
          nodes,
        },
      );

      const startedAt = new Date();
      const error = await assertRejects(
        () => client.flows.read(cloned.flow_id, { skipCache: true }),
        ApiError,
      );
      assertEquals(error.status, 408);

      const readRun = await waitForReadRunCompletion(cloned.flow_id, startedAt);
      assert(readRun.end_time != null, "expected timed out read run to be finished");
    } finally {
      if (cloneIds.length > 0) {
        await deleteFlowV2Rows(cloneIds);
      }
    }
  },
  { sanitizeOps: false, sanitizeResources: false },
);

contractTest(
  "flow contract: startShared still runs flagged flows",
  async () => {
    const startFlowId = await resolveFixtureFlowId("start");
    await withFlowFlags(
      startFlowId,
      { start_shared: true },
      async () => {
        const client = apiClient();
        const run = await client.flows.startShared(startFlowId, {
          inputs: {
            a: 4,
            b: 6,
          },
        });
        const output = await run.output();

        assertEquals(output.toJSObject().c, 10);
      },
    );
  },
);

contractTest(
  "flow contract: startUnverified returns a tokenized run handle",
  async () => {
    const startFlowId = await resolveFixtureFlowId("start");
    await withFlowFlags(
      startFlowId,
      { start_unverified: true, isPublic: true },
      async () => {
        const starterKeypair = web3.Keypair.generate();
        const client = createClient({
          baseUrl: FLOW_SERVER_URL,
          auth: publicKeyAuth(starterKeypair.publicKey),
        });
        const run = await client.flows.startUnverified(startFlowId, {
          inputs: {
            a: 7,
            b: 8,
          },
        });
        const output = await run.output();

        assert(run.token != null);
        assertEquals(output.toJSObject().c, 15);
      },
    );
  },
);
