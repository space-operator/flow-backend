#!/usr/bin/env -S deno run -A

import { load } from "@std/dotenv";
import { encodeBase58 } from "jsr:@std/encoding@^1.0.10/base58";
import { dirname, fromFileUrl, join } from "jsr:@std/path@^1.1.2";
import { parseArgs } from "jsr:@std/cli/parse-args";

const REQUIRED_FIXTURE_FLOW_NAMES = [
  "Add",
  "Deno Add",
  "Collatz",
  "Interflow Instructions",
  "Consts",
  "API Input",
  "Transfer SOL",
  "Collatz-Core",
  "Simple Transfer",
] as const;

function getEnv(key: string): string {
  const value = Deno.env.get(key);
  if (!value) {
    throw new Error(`environment variable ${key} not found`);
  }
  return value;
}

function withTrailingSlash(url: string): string {
  return url.endsWith("/") ? url : `${url}/`;
}

function buildUrl(base: string, pathname: string): string {
  return new URL(pathname, withTrailingSlash(base)).toString();
}

async function readFixtureFile(path: string) {
  const text = await Deno.readTextFile(path);
  return JSON.parse(text);
}

function getFixtureOwnerId(data: unknown): string | undefined {
  if (
    typeof data === "object" &&
    data !== null &&
    "user_id" in data &&
    typeof (data as { user_id?: unknown }).user_id === "string"
  ) {
    return (data as { user_id: string }).user_id;
  }
  return undefined;
}

async function fetchFixtureFlows(
  supabaseUrl: string,
  serviceRoleKey: string,
): Promise<Array<{ name: string; uuid: string }>> {
  const url = new URL("rest/v1/flows", withTrailingSlash(supabaseUrl));
  url.searchParams.set("select", "name,uuid");
  url.searchParams.set(
    "name",
    `in.(${REQUIRED_FIXTURE_FLOW_NAMES.map((name) => `"${name}"`).join(",")})`,
  );

  const response = await fetch(url, {
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
    },
  });

  if (!response.ok) {
    throw new Error(
      `failed to query fixture flows from ${url}: ${response.status} ${await response.text()}`,
    );
  }

  return await response.json();
}

type FlowRow = {
  name: string;
  uuid: string;
  user_id?: string;
  nodes?: unknown;
  start_shared?: boolean;
  start_unverified?: boolean;
  isPublic?: boolean;
};

type FlowV2Flags = {
  uuid: string;
  start_shared: boolean;
  start_unverified: boolean;
  is_public: boolean;
};

type FlowNode = {
  type?: string;
  data?: {
    node_id?: string;
    config?: Record<string, unknown>;
  };
};

async function fetchDetailedFixtureFlows(
  supabaseUrl: string,
  serviceRoleKey: string,
): Promise<FlowRow[]> {
  const url = new URL("rest/v1/flows", withTrailingSlash(supabaseUrl));
  url.searchParams.set(
    "select",
    "name,uuid,user_id,nodes,start_shared,start_unverified,isPublic",
  );
  url.searchParams.set(
    "name",
    `in.(${REQUIRED_FIXTURE_FLOW_NAMES.map((name) => `"${name}"`).join(",")})`,
  );

  const response = await fetch(url, {
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
    },
  });

  if (!response.ok) {
    throw new Error(
      `failed to query detailed fixture flows from ${url}: ${response.status} ${await response.text()}`,
    );
  }

  return await response.json();
}

async function fetchFlowsByIds(
  supabaseUrl: string,
  serviceRoleKey: string,
  ids: string[],
): Promise<Array<{ uuid: string }>> {
  if (ids.length === 0) {
    return [];
  }

  const url = new URL("rest/v1/flows_v2", withTrailingSlash(supabaseUrl));
  url.searchParams.set("select", "uuid");
  url.searchParams.set("uuid", `in.(${ids.map((id) => `"${id}"`).join(",")})`);

  const response = await fetch(url, {
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
    },
  });

  if (!response.ok) {
    throw new Error(
      `failed to query referenced flows from ${url}: ${response.status} ${await response.text()}`,
    );
  }

  return await response.json();
}

async function fetchFlowV2Flags(
  supabaseUrl: string,
  serviceRoleKey: string,
  flowId: string,
): Promise<FlowV2Flags> {
  const url = new URL("rest/v1/flows_v2", withTrailingSlash(supabaseUrl));
  url.searchParams.set(
    "select",
    "uuid,start_shared,start_unverified,is_public",
  );
  url.searchParams.set("uuid", `eq.${flowId}`);

  const response = await fetch(url, {
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
    },
  });

  if (!response.ok) {
    throw new Error(
      `failed to query flows_v2 flags from ${url}: ${response.status} ${await response.text()}`,
    );
  }

  const rows = await response.json() as FlowV2Flags[];
  const row = rows[0];
  if (!row) {
    throw new Error(`missing flows_v2 row for ${flowId}`);
  }
  return row;
}

async function updateFlowV2Flags(
  supabaseUrl: string,
  serviceRoleKey: string,
  flowId: string,
  flags: Partial<Omit<FlowV2Flags, "uuid">>,
): Promise<void> {
  const url = new URL("rest/v1/flows_v2", withTrailingSlash(supabaseUrl));
  url.searchParams.set("uuid", `eq.${flowId}`);

  const response = await fetch(url, {
    method: "PATCH",
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
      "content-type": "application/json",
      prefer: "return=minimal",
    },
    body: JSON.stringify(flags),
  });

  if (!response.ok) {
    throw new Error(
      `failed to update flows_v2 flags for ${flowId}: ${response.status} ${await response.text()}`,
    );
  }
}

async function updateDeployment(
  supabaseUrl: string,
  serviceRoleKey: string,
  deploymentId: string,
  body: Record<string, unknown>,
): Promise<void> {
  const url = new URL("rest/v1/flow_deployments", withTrailingSlash(supabaseUrl));
  url.searchParams.set("id", `eq.${deploymentId}`);

  const response = await fetch(url, {
    method: "PATCH",
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
      "content-type": "application/json",
      prefer: "return=minimal",
    },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    throw new Error(
      `failed to update flow_deployments row for ${deploymentId}: ${response.status} ${await response.text()}`,
    );
  }
}

async function deleteDeployment(
  supabaseUrl: string,
  serviceRoleKey: string,
  deploymentId: string,
): Promise<void> {
  const url = new URL("rest/v1/flow_deployments", withTrailingSlash(supabaseUrl));
  url.searchParams.set("id", `eq.${deploymentId}`);

  const response = await fetch(url, {
    method: "DELETE",
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
      prefer: "return=minimal",
    },
  });

  if (!response.ok) {
    throw new Error(
      `failed to delete flow_deployments row for ${deploymentId}: ${response.status} ${await response.text()}`,
    );
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function getFlowNodes(flow: FlowRow): FlowNode[] {
  if (Array.isArray(flow.nodes)) {
    return flow.nodes as FlowNode[];
  }
  return [];
}

function readTaggedString(value: unknown): string | undefined {
  if (typeof value === "string") {
    return value;
  }
  if (!isRecord(value)) {
    return undefined;
  }
  return typeof value.S === "string" ? value.S : undefined;
}

function collectFixtureIntegrityIssues(flows: FlowRow[]): {
  issues: string[];
  referencedFlowIds: string[];
} {
  const issues: string[] = [];
  const idsByName = new Map(flows.map((flow) => [flow.name, flow.uuid]));
  const referencedFlowIds = new Set<string>();

  for (const flow of flows) {
    const nodes = getFlowNodes(flow);
    for (const node of nodes) {
      const nodeId = node.data?.node_id ?? "";
      const config = isRecord(node.data?.config) ? node.data!.config! : {};
      const looksDeno = node.type === "deno" ||
        nodeId === "deno_script" ||
        nodeId.startsWith("deno_");

      if (looksDeno) {
        const source = typeof config.source === "string"
          ? config.source
          : typeof config.code === "string"
          ? config.code
          : undefined;
        if (!source || source.trim().length === 0) {
          issues.push(
            `"${flow.name}" has Deno node "${nodeId}" without inline source/code`,
          );
        }
      }

      if (nodeId === "interflow" || nodeId === "interflow_instructions") {
        const flowId = readTaggedString(config.flow_id);
        if (!flowId) {
          issues.push(
            `"${flow.name}" has ${nodeId} node without a valid config.flow_id`,
          );
        } else {
          referencedFlowIds.add(flowId);
        }
      }
    }
  }

  if (idsByName.size !== flows.length) {
    issues.push("duplicate fixture flow names found in local database");
  }

  return {
    issues,
    referencedFlowIds: Array.from(referencedFlowIds),
  };
}

async function verifyReferencedFlowsExist(
  supabaseUrl: string,
  serviceRoleKey: string,
  ids: string[],
): Promise<string[]> {
  const rows = await fetchFlowsByIds(supabaseUrl, serviceRoleKey, ids);
  const found = new Set(rows.map((row) => row.uuid));
  return ids.filter((id) => !found.has(id));
}

async function probeUnverifiedStart(
  serverUrl: string,
  supabaseUrl: string,
  serviceRoleKey: string,
  flowId: string,
): Promise<string | undefined> {
  const original = await fetchFlowV2Flags(supabaseUrl, serviceRoleKey, flowId);
  await updateFlowV2Flags(supabaseUrl, serviceRoleKey, flowId, {
    start_unverified: true,
    is_public: true,
  });

  try {
    const pubkey = encodeBase58(crypto.getRandomValues(new Uint8Array(32)));
    const startResponse = await fetch(buildUrl(serverUrl, `flow/start_unverified/${flowId}`), {
      method: "POST",
      headers: {
        authorization: `Bearer ${pubkey}`,
        "content-type": "application/json",
      },
      body: JSON.stringify({
        inputs: {
          a: 1,
          b: 2,
        },
      }),
    });

    if (!startResponse.ok) {
      return `unverified start probe failed for fixture flow ${flowId}: ${startResponse.status} ${await startResponse.text()}`;
    }

    const started = await startResponse.json() as {
      flow_run_id?: string;
      token?: string;
    };
    if (!started.flow_run_id || !started.token) {
      return "unverified start probe returned an unexpected response body";
    }

    const outputResponse = await fetch(
      buildUrl(serverUrl, `flow/output/${started.flow_run_id}`),
      {
        headers: {
          authorization: `Bearer ${started.token}`,
        },
      },
    );
    if (!outputResponse.ok) {
      return `unverified output probe failed for flow run ${started.flow_run_id}: ${outputResponse.status} ${await outputResponse.text()}`;
    }
  } finally {
    await updateFlowV2Flags(supabaseUrl, serviceRoleKey, flowId, {
      start_shared: original.start_shared,
      start_unverified: original.start_unverified,
      is_public: original.is_public,
    });
  }

  return undefined;
}

async function deployFixtureFlow(
  serverUrl: string,
  apiKey: string,
  flowId: string,
): Promise<string> {
  const response = await fetch(buildUrl(serverUrl, `flow/deploy/${flowId}`), {
    method: "POST",
    headers: {
      "x-api-key": apiKey,
    },
  });

  if (!response.ok) {
    throw new Error(
      `fixture deployment probe could not deploy flow ${flowId}: ${response.status} ${await response.text()}`,
    );
  }

  const body = await response.json() as { deployment_id?: string };
  if (!body.deployment_id) {
    throw new Error(
      `fixture deployment probe received an unexpected deploy response for ${flowId}`,
    );
  }
  return body.deployment_id;
}

async function probeAnonymousDeploymentStart(
  serverUrl: string,
  supabaseUrl: string,
  serviceRoleKey: string,
  flowId: string,
): Promise<string | undefined> {
  const apiKey = Deno.env.get("APIKEY");
  if (!apiKey) {
    return "anonymous deployment-start probe skipped because APIKEY is not set";
  }

  let deploymentId: string | undefined;
  try {
    deploymentId = await deployFixtureFlow(serverUrl, apiKey, flowId);
    await updateDeployment(supabaseUrl, serviceRoleKey, deploymentId, {
      start_permission: "Anonymous",
    });

    const pubkey = encodeBase58(crypto.getRandomValues(new Uint8Array(32)));
    const startResponse = await fetch(
      buildUrl(serverUrl, `deployment/start?id=${deploymentId}`),
      {
        method: "POST",
        headers: {
          authorization: `Bearer ${pubkey}`,
          "content-type": "application/json",
        },
        body: JSON.stringify({
          inputs: {
            a: { U: "3" },
            b: { U: "4" },
          },
        }),
      },
    );

    if (!startResponse.ok) {
      return `anonymous deployment-start probe failed for deployment ${deploymentId}: ${startResponse.status} ${await startResponse.text()}`;
    }

    const started = await startResponse.json() as {
      flow_run_id?: string;
      token?: string;
    };
    if (!started.flow_run_id || !started.token) {
      return "anonymous deployment-start probe returned an unexpected response body";
    }

    const outputResponse = await fetch(
      buildUrl(serverUrl, `flow/output/${started.flow_run_id}`),
      {
        headers: {
          authorization: `Bearer ${started.token}`,
        },
      },
    );

    if (!outputResponse.ok) {
      return `anonymous deployment output probe failed for flow run ${started.flow_run_id}: ${outputResponse.status} ${await outputResponse.text()}`;
    }
  } finally {
    if (deploymentId) {
      await deleteDeployment(supabaseUrl, serviceRoleKey, deploymentId).catch((error) => {
        console.warn(
          `Warning: could not clean up probe deployment ${deploymentId}: ${
            error instanceof Error ? error.message : String(error)
          }`,
        );
      });
    }
  }

  return undefined;
}

async function runFixturePreflight(
  serverUrl: string,
  supabaseUrl: string,
  serviceRoleKey: string,
): Promise<void> {
  const flows = await fetchDetailedFixtureFlows(supabaseUrl, serviceRoleKey);
  const { issues, referencedFlowIds } = collectFixtureIntegrityIssues(flows);
  const missingReferencedFlowIds = await verifyReferencedFlowsExist(
    supabaseUrl,
    serviceRoleKey,
    referencedFlowIds,
  );
  for (const missingId of missingReferencedFlowIds) {
    issues.push(`fixture interflow reference missing target flow ${missingId}`);
  }

  const addFlow = flows.find((flow) => flow.name === "Add");
  if (!addFlow) {
    issues.push('fixture preflight could not find the "Add" flow for the unverified-start probe');
  } else {
    const unverifiedIssue = await probeUnverifiedStart(
      serverUrl,
      supabaseUrl,
      serviceRoleKey,
      addFlow.uuid,
    );
    if (unverifiedIssue) {
      issues.push(unverifiedIssue);
    }
  }

  if (!addFlow) {
    issues.push('fixture preflight could not find the "Add" flow for the anonymous deployment-start probe');
  } else {
    const deploymentIssue = await probeAnonymousDeploymentStart(
      serverUrl,
      supabaseUrl,
      serviceRoleKey,
      addFlow.uuid,
    );
    if (deploymentIssue) {
      issues.push(deploymentIssue);
    }
  }

  if (issues.length > 0) {
    throw new Error(
      `fixture preflight failed:\n- ${issues.join("\n- ")}`,
    );
  }

  console.log("Fixture preflight passed.");
}

async function listMissingFixtureFlows(
  supabaseUrl: string,
  serviceRoleKey: string,
): Promise<string[]> {
  const rows = await fetchFixtureFlows(supabaseUrl, serviceRoleKey);
  const present = new Set(rows.map((row) => row.name));
  return REQUIRED_FIXTURE_FLOW_NAMES.filter((name) => !present.has(name));
}

async function importFixtureData(
  serverUrl: string,
  serviceRoleKey: string,
  data: unknown,
) {
  const response = await fetch(buildUrl(serverUrl, "data/import"), {
    method: "POST",
    headers: {
      authorization: `Bearer ${serviceRoleKey}`,
      "content-type": "application/json",
    },
    body: JSON.stringify(data),
  });

  if (response.ok) {
    return;
  }

  const text = await response.text();
  if (response.status === 404) {
    throw new Error(
      `fixture import endpoint is unavailable at ${buildUrl(serverUrl, "data/import")}. ` +
        "Make sure your local flow-server is running with the default import feature enabled.",
    );
  }

  throw new Error(
    `fixture import failed: ${response.status} ${text}`,
  );
}

async function verifyApiKey(
  serverUrl: string,
  expectedUserId?: string,
): Promise<void> {
  const apiKey = Deno.env.get("APIKEY");
  if (!apiKey) {
    console.warn(
      "Warning: APIKEY is not set. Owner-authenticated client tests will still fail after fixture bootstrap.",
    );
    return;
  }

  const response = await fetch(buildUrl(serverUrl, "apikey/info"), {
    headers: {
      "x-api-key": apiKey,
    },
  });

  if (!response.ok) {
    console.warn(
      `Warning: APIKEY did not validate against ${buildUrl(serverUrl, "apikey/info")} (${response.status}). ` +
        "The fixture flows may be present, but owner-authenticated tests will still fail until APIKEY matches the imported owner.",
    );
    return;
  }

  if (!expectedUserId) {
    return;
  }

  try {
    const info = await response.json() as { user_id?: unknown };
    if (info.user_id !== expectedUserId) {
      console.warn(
        `Warning: APIKEY belongs to user ${String(info.user_id)} but the imported fixture owner is ${expectedUserId}. ` +
          "Private fixture flows will not be visible to the test client until APIKEY matches the imported owner.",
      );
    }
  } catch {
    console.warn(
      "Warning: could not parse /apikey/info response while verifying fixture owner access.",
    );
  }
}

async function main() {
  await load({ export: true });

  const args = parseArgs(Deno.args, {
    boolean: ["force", "verify", "preflight-only"],
    string: ["file", "server", "supabase-url"],
    default: {
      verify: true,
      "preflight-only": false,
    },
  });

  const dockerDir = dirname(fromFileUrl(import.meta.url));
  const file = args.file ?? join(dockerDir, "export.json");
  const serverUrl = args.server ?? Deno.env.get("FLOW_SERVER_URL") ??
    "http://127.0.0.1:8080";
  const supabaseUrl = args["supabase-url"] ?? Deno.env.get("SUPABASE_URL") ??
    "http://127.0.0.1:8000";
  const serviceRoleKey = getEnv("SERVICE_ROLE_KEY");
  const expectedUserId = getFixtureOwnerId(await readFixtureFile(file));

  if (!args.force) {
    const missing = await listMissingFixtureFlows(supabaseUrl, serviceRoleKey);
    if (missing.length === 0) {
      console.log("Local test fixtures already present.");
      if (args.verify) {
        await runFixturePreflight(serverUrl, supabaseUrl, serviceRoleKey);
      }
      await verifyApiKey(serverUrl, expectedUserId);
      return;
    }

    if (args["preflight-only"]) {
      throw new Error(
        `fixture preflight requested, but local fixture flows are missing: ${missing.join(", ")}. ` +
          "Run the bootstrap without --preflight-only first.",
      );
    }

    console.log(`Missing fixture flows: ${missing.join(", ")}`);
  }

  if (args["preflight-only"]) {
    console.log("Running fixture preflight without importing data.");
    if (args.verify) {
      await runFixturePreflight(serverUrl, supabaseUrl, serviceRoleKey);
    }
    await verifyApiKey(serverUrl, expectedUserId);
    return;
  }

  console.log(`Reading fixture export from ${file}`);
  const data = await readFixtureFile(file);

  console.log(`Importing fixtures into ${serverUrl}`);
  await importFixtureData(serverUrl, serviceRoleKey, data);

  if (args.verify) {
    const missing = await listMissingFixtureFlows(supabaseUrl, serviceRoleKey);
    if (missing.length > 0) {
      throw new Error(
        `fixture bootstrap finished, but flows are still missing: ${missing.join(", ")}`,
      );
    }
  }

  const rows = await fetchFixtureFlows(supabaseUrl, serviceRoleKey);
  console.log("Fixture flows available:");
  for (const row of rows.sort((a, b) => a.name.localeCompare(b.name))) {
    console.log(`- ${row.name}: ${row.uuid}`);
  }

  if (args.verify) {
    await runFixturePreflight(serverUrl, supabaseUrl, serviceRoleKey);
  }

  await verifyApiKey(serverUrl, expectedUserId);
}

await main();
