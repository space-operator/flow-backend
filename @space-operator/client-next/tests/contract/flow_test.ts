import { createClient, publicKeyAuth } from "../../src/mod.ts";
import { assert, assertEquals } from "@std/assert";
import {
  apiClient,
  contractTest,
  FLOW_SERVER_URL,
  getEnv,
  ownerSupabase,
  resolveFixtureFlowId,
  SUPABASE_URL,
  web3,
} from "./_shared.ts";

type FlowV2Flags = {
  start_shared?: boolean;
  start_unverified?: boolean;
  is_public?: boolean;
};

async function requestFlowV2<T>(
  method: "GET" | "PATCH",
  query: string,
  body?: FlowV2Flags,
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

async function withFlowFlags(
  flowId: string,
  flags: {
    start_shared?: boolean;
    start_unverified?: boolean;
    isPublic?: boolean;
  },
  fn: () => Promise<void>,
) {
  const query = `?uuid=eq.${flowId}&select=uuid,start_shared,start_unverified,is_public`;
  const [original] = await requestFlowV2<Array<{
    uuid: string;
    start_shared: boolean;
    start_unverified: boolean;
    is_public: boolean;
  }>>("GET", query);
  if (!original) {
    throw new Error(`missing flows_v2 row for ${flowId}`);
  }

  const [updated] = await requestFlowV2<Array<{
    start_shared: boolean;
    start_unverified: boolean;
    is_public: boolean;
  }>>("PATCH", query, {
    start_shared: flags.start_shared,
    start_unverified: flags.start_unverified,
    is_public: flags.isPublic,
  });
  for (const [key, value] of Object.entries({
    start_shared: flags.start_shared,
    start_unverified: flags.start_unverified,
    is_public: flags.isPublic,
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
    const { supabase } = await ownerSupabase();
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
        const cleanup = await supabase.from("flows").delete().in(
          "uuid",
          cloneIds,
        );
        if (cleanup.error) {
          throw new Error(JSON.stringify(cleanup.error));
        }
      }
    }
  },
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
