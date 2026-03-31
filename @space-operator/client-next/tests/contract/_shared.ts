import { load } from "@std/dotenv";
import { createClient as createSupabaseClient } from "@supabase/supabase-js";
import * as nacl from "tweetnacl";
import {
  apiKeyAuth,
  bearerAuth,
  createClient,
  type Database,
  type ServiceInfoOutput,
  Value,
  web3,
} from "../../src/mod.ts";

await load({
  export: true,
  envPath: decodeURIComponent(
    new URL("../../../../.env", import.meta.url).pathname,
  ),
});

export const RUN_CONTRACT_TESTS = Deno.env.get(
  "RUN_SPACE_OPERATOR_CONTRACT_TESTS",
) === "1";
export const RUN_E2E_TESTS = RUN_CONTRACT_TESTS ||
  Deno.env.get("RUN_SPACE_OPERATOR_E2E_TESTS") === "1";
export const RUN_EXPORT_TESTS = RUN_E2E_TESTS &&
  Deno.env.get("RUN_SPACE_OPERATOR_EXPORT_TESTS") === "1";
export const RUN_X402_TESTS = RUN_E2E_TESTS &&
  Deno.env.get("RUN_SPACE_OPERATOR_X402_TESTS") === "1";

export const FLOW_SERVER_URL = Deno.env.get("FLOW_SERVER_URL") ??
  "http://localhost:8080";
export const SUPABASE_URL = Deno.env.get("SUPABASE_URL") ??
  "http://localhost:8000";
export const ANON_KEY = Deno.env.get("ANON_KEY") ?? "";
export const TEST_WEBHOOK_URL = Deno.env.get("FLOW_TEST_WEBHOOK_URL") ??
  "http://webhook/webhook";
const SERVICE_ROLE_KEY = Deno.env.get("SERVICE_ROLE_KEY") ?? "";
const ALLOW_LEGACY_FIXTURE_UUID_FALLBACK = Deno.env.get(
  "ALLOW_LEGACY_FIXTURE_UUID_FALLBACK",
) === "1";
const ALLOW_FIXTURE_FLOW_ENV_OVERRIDE = Deno.env.get(
  "ALLOW_FIXTURE_FLOW_ENV_OVERRIDE",
) === "1";

const FIXTURE_FLOW_IDS = {
  start: "6c949718-69e2-47c1-8b93-d56b8e34ec51",
  deno: "c349c074-0f4f-41bd-976d-d8df32ba867a",
  interflow: "b3c95f36-2a1c-4e33-be2a-28758a0c4b9d",
  interflowInstructions: "69401e5a-375e-49d0-bb95-33c9d70eb582",
  consts: "27b35933-7165-4da5-a2ea-a6342bbb3da7",
  apiInput: "78a7e826-7697-48cb-a2c0-67ad1be4e970",
  deployRun: "92b480ad-1a18-4a52-a459-4d5420890272",
  deployDelete: "102244df-74aa-4f77-a556-d9d279c64655",
  deployAction: "9647ba16-de20-4209-9056-1a3dd8c2d6ab",
  deploySimple: "6c949718-69e2-47c1-8b93-d56b8e34ec51",
  x402: "b3c95f36-2a1c-4e33-be2a-28758a0c4b9d",
} as const;

const FIXTURE_FLOW_NAMES = {
  start: "Add",
  deno: "Deno Add",
  interflow: "Collatz",
  interflowInstructions: "Interflow Instructions",
  consts: "Consts",
  apiInput: "API Input",
  deployRun: "Transfer SOL",
  deployDelete: "Collatz-Core",
  deployAction: "Simple Transfer",
  deploySimple: "Add",
  x402: "Collatz",
} as const;

const FIXTURE_ENV_KEYS = {
  start: "START_FLOW_ID",
  deno: "DENO_FLOW_ID",
  interflow: "INTERFLOW_FLOW_ID",
  interflowInstructions: "INTERFLOW_INSTRUCTIONS_FLOW_ID",
  consts: "CONSTS_FLOW_ID",
  apiInput: "API_INPUT_FLOW_ID",
  deployRun: "DEPLOY_RUN_FLOW_ID",
  deployDelete: "DEPLOY_DELETE_FLOW_ID",
  deployAction: "DEPLOY_ACTION_FLOW_ID",
  deploySimple: "DEPLOY_SIMPLE_FLOW_ID",
  x402: "X402_FLOW_ID",
} as const;

type FixtureFlowKey = keyof typeof FIXTURE_FLOW_IDS;

const fixtureFlowIdCache = new Map<FixtureFlowKey, Promise<string>>();
const ignoredFixtureEnvOverrideWarnings = new Set<FixtureFlowKey>();
let serviceInfoPromise: Promise<ServiceInfoOutput> | undefined;

type VisibleFlowRow = {
  uuid: string;
};

export function contractTest(
  name: string,
  fn: () => Promise<void>,
  options: Partial<Omit<Deno.TestDefinition, "name" | "fn">> = {},
) {
  Deno.test({
    name,
    ignore: !RUN_E2E_TESTS,
    sanitizeOps: false,
    sanitizeResources: false,
    fn,
    ...options,
  });
}

export function getEnv(key: string): string {
  const value = Deno.env.get(key);
  if (!value) {
    const fallbacks = [
      ...(key === "KEYPAIR" ? ["keypair", "OWNER_KEYPAIR"] : []),
      ...(key === "OWNER_KEYPAIR" ? ["KEYPAIR", "keypair"] : []),
    ];
    for (const fallback of fallbacks) {
      const alternate = Deno.env.get(fallback);
      if (alternate) {
        return alternate;
      }
    }
    throw new Error(`missing env ${key}`);
  }
  return value;
}

export async function resolveFixtureFlowId(
  key: FixtureFlowKey,
): Promise<string> {
  const cached = fixtureFlowIdCache.get(key);
  if (cached) {
    return await cached;
  }

  const promise = (async () => {
    const override = Deno.env.get(FIXTURE_ENV_KEYS[key]);
    if (override && ALLOW_FIXTURE_FLOW_ENV_OVERRIDE) {
      return override;
    }
    if (override && !ignoredFixtureEnvOverrideWarnings.has(key)) {
      ignoredFixtureEnvOverrideWarnings.add(key);
      console.warn(
        `Ignoring ${FIXTURE_ENV_KEYS[key]}=${override}. ` +
          "Set ALLOW_FIXTURE_FLOW_ENV_OVERRIDE=1 to force fixture flow IDs from env.",
      );
    }

    let rows: VisibleFlowRow[];
    try {
      rows = await requestOwnerFlowsV2<VisibleFlowRow[]>(
        `?select=uuid&name=eq.${encodeURIComponent(FIXTURE_FLOW_NAMES[key])}&order=updated_at.desc&limit=1`,
      );
    } catch (error) {
      if (ALLOW_LEGACY_FIXTURE_UUID_FALLBACK) {
        return FIXTURE_FLOW_IDS[key];
      }
      throw new Error(
        `could not resolve fixture flow "${FIXTURE_FLOW_NAMES[key]}" through owner-visible flows. ` +
          `Fix APIKEY/bootstrap or set ${FIXTURE_ENV_KEYS[key]}. ${
            error instanceof Error ? error.message : String(error)
          }`,
      );
    }

    const result = rows[0];
    if (result?.uuid) {
      return result.uuid;
    }

    if (ALLOW_LEGACY_FIXTURE_UUID_FALLBACK) {
      const fallback = await requestOwnerFlowsV2<VisibleFlowRow[]>(
        `?select=uuid&uuid=eq.${FIXTURE_FLOW_IDS[key]}&limit=1`,
      );

      if (fallback[0]?.uuid) {
        return fallback[0].uuid;
      }
    }

    throw new Error(
      `missing fixture flow "${FIXTURE_FLOW_NAMES[key]}". Run \`deno task test:bootstrap-fixtures\` from @space-operator/client-next or set ${FIXTURE_ENV_KEYS[key]}.${
        ALLOW_LEGACY_FIXTURE_UUID_FALLBACK
          ? ` Legacy fallback UUID ${FIXTURE_FLOW_IDS[key]} was not visible either.`
          : ""
      }`,
    );
  })();

  fixtureFlowIdCache.set(key, promise);
  return await promise;
}

export function apiClient() {
  return createClient({
    baseUrl: FLOW_SERVER_URL,
    auth: apiKeyAuth(getEnv("APIKEY")),
  });
}

export async function serviceInfo() {
  serviceInfoPromise ??= createClient({ baseUrl: FLOW_SERVER_URL }).service
    .info();
  return await serviceInfoPromise;
}

export async function ownerSession() {
  return await apiClient().auth.claimToken();
}

async function requestOwnerFlowsV2<T>(query: string): Promise<T> {
  const session = await ownerSession();
  const response = await fetch(`${SUPABASE_URL}/rest/v1/flows_v2${query}`, {
    headers: {
      apikey: ANON_KEY,
      authorization: `Bearer ${session.access_token}`,
    },
  });
  const text = await response.text();
  if (!response.ok) {
    throw new Error(
      `flows_v2 query failed: ${response.status} ${text || response.statusText}`,
    );
  }
  return JSON.parse(text) as T;
}

export async function ownerUserId() {
  return (await apiClient().apiKeys.info()).user_id;
}

export async function ownerBearerClient() {
  const session = await ownerSession();
  return {
    session,
    client: createClient({
      baseUrl: FLOW_SERVER_URL,
      auth: bearerAuth(session.access_token),
    }),
  };
}

export async function ownerSupabase() {
  const session = await ownerSession();
  const client = supabase();
  await client.auth.setSession(session as never);
  return { session, supabase: client };
}

export function supabase() {
  return createSupabaseClient<Database>(SUPABASE_URL, ANON_KEY, {
    auth: { autoRefreshToken: false },
  });
}

export function adminSupabase() {
  if (!SERVICE_ROLE_KEY) {
    throw new Error("missing env SERVICE_ROLE_KEY");
  }
  return createSupabaseClient<Database>(SUPABASE_URL, SERVICE_ROLE_KEY, {
    auth: { autoRefreshToken: false },
  });
}

export async function serviceSupabase() {
  const info = await serviceInfo();
  return createSupabaseClient<Database>(info.supabase_url, info.anon_key, {
    auth: { autoRefreshToken: false },
  });
}

export async function signText(
  keypair: web3.Keypair,
  message: string,
): Promise<Uint8Array> {
  return nacl.default.sign.detached(
    new TextEncoder().encode(message),
    keypair.secretKey,
  );
}

export async function checkNoErrors(
  flowRunId: string,
  accessToken: string,
) {
  const sup = supabase();
  await sup.auth.setSession({
    access_token: accessToken,
    refresh_token: "unused",
  } as never);

  const nodeErrors = await sup
    .from("node_run")
    .select("errors")
    .eq("flow_run_id", flowRunId)
    .not("errors", "is", "null");
  if (nodeErrors.error) {
    throw new Error(JSON.stringify(nodeErrors.error));
  }

  const flowErrors = await sup
    .from("flow_run")
    .select("errors")
    .eq("id", flowRunId)
    .not("errors", "is", "null");
  if (flowErrors.error) {
    throw new Error(JSON.stringify(flowErrors.error));
  }

  const errors = [
    ...flowErrors.data.flatMap((row) => row.errors ?? []),
    ...nodeErrors.data.flatMap((row) => row.errors ?? []),
  ];
  if (errors.length > 0) {
    throw new Error(JSON.stringify(errors));
  }
}

export async function checkNoErrorsAdmin(flowRunId: string) {
  const sup = adminSupabase();

  const nodeErrors = await sup
    .from("node_run")
    .select("errors")
    .eq("flow_run_id", flowRunId)
    .not("errors", "is", "null");
  if (nodeErrors.error) {
    throw new Error(JSON.stringify(nodeErrors.error));
  }

  const flowErrors = await sup
    .from("flow_run")
    .select("errors")
    .eq("id", flowRunId)
    .not("errors", "is", "null");
  if (flowErrors.error) {
    throw new Error(JSON.stringify(flowErrors.error));
  }

  const errors = [
    ...flowErrors.data.flatMap((row) => row.errors ?? []),
    ...nodeErrors.data.flatMap((row) => row.errors ?? []),
  ];
  if (errors.length > 0) {
    throw new Error(JSON.stringify(errors));
  }
}

export function fixApiInputUrl(url: string) {
  const submit = new URL(url);
  return new URL(`${submit.pathname}${submit.search}`, FLOW_SERVER_URL)
    .toString();
}

type WebhookServerHandle = {
  url: string;
  close: () => Promise<void>;
};

async function startLocalWebhookEchoServer(): Promise<WebhookServerHandle> {
  const server = Deno.serve({ hostname: "127.0.0.1", port: 0 }, async (request) => {
    const payload = await request.json() as {
      url?: string;
      extra?: { output?: unknown };
    };
    if (typeof payload.url !== "string") {
      return new Response("missing url", { status: 400 });
    }
    await fetch(payload.url, {
      method: "POST",
      headers: [["content-type", "application/json"]],
      body: JSON.stringify({
        value: payload.extra?.output ?? { S: "hello" },
      }),
    });
    return new Response("ok");
  });

  const addr = server.addr as Deno.NetAddr;
  return {
    url: `http://127.0.0.1:${addr.port}/webhook`,
    close: async () => {
      await server.shutdown();
    },
  };
}

export async function withWebhookUrl<T>(
  fn: (url: string) => Promise<T>,
): Promise<T> {
  if (Deno.env.get("FLOW_TEST_WEBHOOK_URL")) {
    return await fn(TEST_WEBHOOK_URL);
  }

  const server = await startLocalWebhookEchoServer();
  try {
    return await fn(server.url);
  } finally {
    await server.close();
  }
}

export function randomName(prefix: string) {
  return `${prefix}-${crypto.randomUUID()}`;
}

export function randomStoreName(prefix = "e2e") {
  return `${prefix}_${crypto.randomUUID().replaceAll("-", "_")}`;
}

export { Value, web3 };
