#!/usr/bin/env -S deno run -A

import { load } from "@std/dotenv";
import { decodeBase58, encodeBase58 } from "jsr:@std/encoding@^1.0.10/base58";
import { dirname, fromFileUrl, join } from "jsr:@std/path@^1.1.2";
import { parseArgs } from "jsr:@std/cli/parse-args";
import { blake3 } from "npm:@noble/hashes@^1.8.0/blake3";

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

const LOCAL_TEST_ENV_FILE = ".flow-test.env";
const LOCAL_API_KEY_NAME = "local-e2e-bootstrap";

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

function isLoopbackUrl(raw: string): boolean {
  try {
    const url = new URL(raw);
    return url.hostname === "127.0.0.1" || url.hostname === "localhost";
  } catch {
    return false;
  }
}

function base64UrlEncode(bytes: Uint8Array, pad: boolean): string {
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  const encoded = btoa(binary).replaceAll("+", "-").replaceAll("/", "_");
  return pad ? encoded : encoded.replace(/=+$/u, "");
}

function generateApiKeyMaterial(): {
  fullKey: string;
  keyHash: string;
  trimmedKey: string;
} {
  const random = crypto.getRandomValues(new Uint8Array(32));
  const fullKey = `b3-${base64UrlEncode(random, false)}`;
  const keyHash = base64UrlEncode(blake3(new TextEncoder().encode(fullKey)), true);
  return {
    fullKey,
    keyHash,
    trimmedKey: `*****${fullKey.slice(-5)}`,
  };
}

function configuredWebhookUrl(serverUrl: string): string | undefined {
  return Deno.env.get("FLOW_TEST_WEBHOOK_URL") ??
    (isLoopbackUrl(serverUrl) ? "http://webhook/webhook" : undefined);
}

function normalizeApiInputUrl(url: string, serverUrl: string): string {
  const submit = new URL(url);
  return new URL(`${submit.pathname}${submit.search}`, withTrailingSlash(serverUrl))
    .toString();
}

function toWsUrl(base: string): string {
  const url = new URL(base);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  url.pathname = `${url.pathname.replace(/\/$/, "")}/ws`;
  return url.toString();
}

async function readFixtureFile(path: string) {
  const text = await Deno.readTextFile(path);
  return JSON.parse(text);
}

type FixtureExport = Record<string, unknown> & {
  user_id?: string;
  users?: string;
  identities?: string;
  pubkey_whitelists?: string;
  users_public?: string;
  wallets?: string;
  apikeys?: string;
  user_quotas?: string;
  kvstore?: string;
  kvstore_metadata?: string;
  flows?: string;
  node_definitions?: string;
};

type SemicolonTable = {
  header: string[];
  rows: string[][];
};

type WalletTarget = {
  walletId: number;
  publicKey: string;
};

function parseSemicolonLine(line: string): string[] {
  const values: string[] = [];
  let current = "";
  let inQuote = false;
  for (let i = 0; i < line.length; i += 1) {
    const char = line[i];
    if (char === "'") {
      if (inQuote && line[i + 1] === "'") {
        current += "'";
        i += 1;
      } else {
        inQuote = !inQuote;
      }
      continue;
    }
    if (char === ";" && !inQuote) {
      values.push(current);
      current = "";
      continue;
    }
    current += char;
  }
  values.push(current);
  return values;
}

function parseSemicolonTable(text: string): SemicolonTable {
  const trimmed = text.trimEnd();
  const lines = trimmed.length === 0 ? [] : trimmed.split("\n");
  if (lines.length === 0) {
    return { header: [], rows: [] };
  }
  return {
    header: parseSemicolonLine(lines[0]),
    rows: lines.slice(1).filter(Boolean).map(parseSemicolonLine),
  };
}

function serializeSemicolonField(value: string): string {
  if (
    value.includes(";") || value.includes("\n") || value.includes("\r") ||
    value.includes("'")
  ) {
    return `'${value.replaceAll("'", "''")}'`;
  }
  return value;
}

function serializeSemicolonTable(table: SemicolonTable): string {
  if (table.header.length === 0) {
    return "";
  }
  const lines = [
    table.header.map(serializeSemicolonField).join(";"),
    ...table.rows.map((row) => row.map((value) => serializeSemicolonField(value)).join(";")),
  ];
  return `${lines.join("\n")}\n`;
}

function emptySemicolonTable(text: string): string {
  const parsed = parseSemicolonTable(text);
  return serializeSemicolonTable({
    header: parsed.header,
    rows: [],
  });
}

function rewriteTaggedFlowIds(value: unknown, flowIdMap: Map<string, string>): unknown {
  if (Array.isArray(value)) {
    return value.map((entry) => rewriteTaggedFlowIds(entry, flowIdMap));
  }
  if (!isRecord(value)) {
    return value;
  }

  const output: Record<string, unknown> = {};
  for (const [key, child] of Object.entries(value)) {
    if (key === "flow_id") {
      if (typeof child === "string") {
        output[key] = flowIdMap.get(child) ?? child;
        continue;
      }
      if (isRecord(child) && typeof child.S === "string") {
        output[key] = {
          ...child,
          S: flowIdMap.get(child.S) ?? child.S,
        };
        continue;
      }
    }
    output[key] = rewriteTaggedFlowIds(child, flowIdMap);
  }
  return output;
}

function rewriteWalletConfig(value: unknown, walletTarget?: WalletTarget): unknown {
  if (Array.isArray(value)) {
    return value.map((entry) => rewriteWalletConfig(entry, walletTarget));
  }
  if (!isRecord(value)) {
    return value;
  }

  const nestedData = isRecord(value.data) ? { ...value.data } : undefined;
  const nodeId = typeof value.node_id === "string"
    ? value.node_id
    : typeof nestedData?.node_id === "string"
    ? nestedData.node_id
    : undefined;
  if (nodeId !== "wallet") {
    const output: Record<string, unknown> = {};
    for (const [key, child] of Object.entries(value)) {
      output[key] = rewriteWalletConfig(child, walletTarget);
    }
    return output;
  }

  const rootConfig = isRecord(value.config) ? { ...value.config } : undefined;
  const nestedConfig = isRecord(nestedData?.config) ? { ...nestedData.config } : undefined;
  const config = nestedConfig ?? rootConfig;
  if (!config || !walletTarget) {
    return {
      ...value,
      ...(nestedData ? { data: nestedData } : {}),
      ...(rootConfig ? { config: rootConfig } : {}),
    };
  }

  config.wallet_id = { U: walletTarget.walletId.toString() };
  config.public_key = { B3: walletTarget.publicKey };
  if (nestedData) {
    nestedData.config = config;
    return {
      ...value,
      data: nestedData,
    };
  }
  return {
    ...value,
    config,
  };
}

function rewriteFixtureFlowRows(
  flowsCsv: string,
  targetUserId: string,
  walletTarget?: WalletTarget,
): {
  csv: string;
  flowIdMap: Map<string, string>;
} {
  const table = parseSemicolonTable(flowsCsv);
  const uuidIndex = table.header.indexOf("uuid");
  const userIdIndex = table.header.indexOf("user_id");
  const updatedAtIndex = table.header.indexOf("updated_at");
  const parentFlowIndex = table.header.indexOf("parent_flow");
  const nodesIndex = table.header.indexOf("nodes");

  if (uuidIndex === -1 || userIdIndex === -1 || nodesIndex === -1) {
    throw new Error("fixture flows export is missing required columns");
  }

  const flowIdMap = new Map<string, string>();
  for (const row of table.rows) {
    const uuid = row[uuidIndex];
    if (uuid) {
      flowIdMap.set(uuid, crypto.randomUUID());
    }
  }

  const rewrittenRows = table.rows.map((row) => {
    const next = [...row];
    const oldUuid = next[uuidIndex];
    next[userIdIndex] = targetUserId;
    next[uuidIndex] = flowIdMap.get(oldUuid) ?? oldUuid;

    if (updatedAtIndex !== -1) {
      next[updatedAtIndex] = new Date().toISOString();
    }

    if (parentFlowIndex !== -1 && next[parentFlowIndex]) {
      next[parentFlowIndex] = flowIdMap.get(next[parentFlowIndex]) ?? next[parentFlowIndex];
    }

    const nodesText = next[nodesIndex];
    if (nodesText) {
      const nodes = JSON.parse(nodesText);
      const rewrittenFlowIds = rewriteTaggedFlowIds(nodes, flowIdMap);
      next[nodesIndex] = JSON.stringify(rewriteWalletConfig(rewrittenFlowIds, walletTarget));
    }

    return next;
  });

  return {
    csv: serializeSemicolonTable({
      header: table.header,
      rows: rewrittenRows,
    }),
    flowIdMap,
  };
}

function prepareFixtureImportData(
  rawData: FixtureExport,
  targetUserId?: string,
  walletTarget?: WalletTarget,
): {
  data: FixtureExport;
  importedUserId: string | undefined;
  notes: string[];
} {
  if (!targetUserId || typeof rawData.flows !== "string") {
    return {
      data: rawData,
      importedUserId: getFixtureOwnerId(rawData),
      notes: [],
    };
  }

  const rewritten = { ...rawData };
  const notes = [
    `rewriting fixture flows to APIKEY owner ${targetUserId}`,
    "importing flow data only to avoid auth/api-key conflicts with an existing local user",
  ];
  if (walletTarget) {
    notes.push(
      `rewriting wallet nodes to wallet ${walletTarget.walletId} (${walletTarget.publicKey})`,
    );
  }

  const { csv } = rewriteFixtureFlowRows(rawData.flows, targetUserId, walletTarget);
  rewritten.user_id = targetUserId;
  rewritten.flows = csv;

  for (
    const key of [
      "users",
      "identities",
      "pubkey_whitelists",
      "users_public",
      "wallets",
      "apikeys",
      "user_quotas",
      "kvstore",
      "kvstore_metadata",
      "node_definitions",
    ] as const
  ) {
    const table = rawData[key];
    if (typeof table === "string") {
      rewritten[key] = emptySemicolonTable(table);
    }
  }

  return {
    data: rewritten,
    importedUserId: targetUserId,
    notes,
  };
}

type FlowsV2InsertRow = {
  uuid: string;
  user_id: string;
  name: string;
  description: string;
  is_public: boolean;
  gg_marketplace: boolean;
  created_at?: string;
  updated_at?: string;
  nodes: unknown;
  edges: unknown;
  viewport: unknown;
  environment: unknown;
  guide: unknown;
  instructions_bundling: unknown;
  current_network: unknown;
  start_shared: boolean;
  start_unverified: boolean;
  parent_flow: string | null;
  meta_nodes: unknown;
  default_viewport: unknown;
};

function parseBooleanField(value: string | undefined, fallback = false): boolean {
  if (value === undefined || value === "") {
    return fallback;
  }
  return value === "t" || value === "true";
}

function parseJsonField(value: string | undefined, fallback: unknown): unknown {
  if (value === undefined || value === "") {
    return fallback;
  }
  return JSON.parse(value);
}

function parseOptionalString(value: string | undefined): string | null {
  return value && value.length > 0 ? value : null;
}

function flowsCsvToInsertRows(flowsCsv: string): FlowsV2InsertRow[] {
  const table = parseSemicolonTable(flowsCsv);
  const index = new Map(table.header.map((name, i) => [name, i]));
  const required = [
    "uuid",
    "user_id",
    "name",
    "description",
    "is_public",
    "gg_marketplace",
    "nodes",
    "edges",
    "viewport",
    "environment",
    "instructions_bundling",
    "current_network",
    "start_shared",
    "start_unverified",
    "meta_nodes",
    "default_viewport",
  ];
  for (const key of required) {
    if (!index.has(key)) {
      throw new Error(`fixture flows export is missing required column ${key}`);
    }
  }

  const read = (row: string[], key: string): string | undefined => {
    const idx = index.get(key);
    return idx === undefined ? undefined : row[idx];
  };

  return table.rows.map((row) => ({
    uuid: read(row, "uuid") ?? crypto.randomUUID(),
    user_id: read(row, "user_id") ?? "",
    name: read(row, "name") ?? "",
    description: read(row, "description") ?? "",
    is_public: parseBooleanField(read(row, "is_public")),
    gg_marketplace: parseBooleanField(read(row, "gg_marketplace")),
    created_at: read(row, "created_at") || undefined,
    updated_at: read(row, "updated_at") || undefined,
    nodes: parseJsonField(read(row, "nodes"), []),
    edges: parseJsonField(read(row, "edges"), []),
    viewport: parseJsonField(read(row, "viewport"), { x: 0, y: 0, zoom: 1 }),
    environment: parseJsonField(read(row, "environment"), {}),
    guide: parseJsonField(read(row, "guide"), null),
    instructions_bundling: parseJsonField(read(row, "instructions_bundling"), "Off"),
    current_network: parseJsonField(read(row, "current_network"), {}),
    start_shared: parseBooleanField(read(row, "start_shared")),
    start_unverified: parseBooleanField(read(row, "start_unverified")),
    parent_flow: parseOptionalString(read(row, "parent_flow")),
    meta_nodes: parseJsonField(read(row, "meta_nodes"), []),
    default_viewport: parseJsonField(read(row, "default_viewport"), { x: 0, y: 0, zoom: 1 }),
  }));
}

function shouldUseRestImportFallback(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return message.includes("auth.disable_users_triggers()") ||
    message.includes("disable trigger") ||
    message.includes("permission denied");
}

async function importFixtureFlowsViaRest(
  supabaseUrl: string,
  serviceRoleKey: string,
  flowsCsv: string,
): Promise<void> {
  const rows = flowsCsvToInsertRows(flowsCsv);
  const targetUserId = rows[0]?.user_id;
  if (targetUserId) {
    const deleteUrl = new URL("rest/v1/flows_v2", withTrailingSlash(supabaseUrl));
    deleteUrl.searchParams.set("user_id", `eq.${targetUserId}`);
    deleteUrl.searchParams.set(
      "name",
      `in.(${[...new Set(rows.map((row) => row.name))].map((name) => `"${name}"`).join(",")})`,
    );
    const deleteResponse = await fetch(deleteUrl, {
      method: "DELETE",
      headers: {
        apikey: serviceRoleKey,
        authorization: `Bearer ${serviceRoleKey}`,
        prefer: "return=minimal",
      },
    });

    if (!deleteResponse.ok) {
      throw new Error(
        `flows_v2 REST cleanup failed: ${deleteResponse.status} ${await deleteResponse.text()}`,
      );
    }
  }

  const response = await fetch(new URL("rest/v1/flows_v2", withTrailingSlash(supabaseUrl)), {
    method: "POST",
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
      "content-type": "application/json",
      prefer: "return=minimal",
    },
    body: JSON.stringify(rows),
  });

  if (!response.ok) {
    throw new Error(
      `flows_v2 REST fixture import failed: ${response.status} ${await response.text()}`,
    );
  }
}

function resolveOptionalEnv(key: string, fallbacks: string[] = []): string | undefined {
  const direct = Deno.env.get(key);
  if (direct) {
    return direct;
  }
  for (const fallback of fallbacks) {
    const value = Deno.env.get(fallback);
    if (value) {
      return value;
    }
  }
  return undefined;
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
  userId?: string,
): Promise<Array<{ name: string; uuid: string }>> {
  const url = new URL("rest/v1/flows_v2", withTrailingSlash(supabaseUrl));
  url.searchParams.set("select", "name,uuid");
  url.searchParams.set(
    "name",
    `in.(${REQUIRED_FIXTURE_FLOW_NAMES.map((name) => `"${name}"`).join(",")})`,
  );
  if (userId) {
    url.searchParams.set("user_id", `eq.${userId}`);
  }

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
  updated_at?: string;
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
  userId?: string,
): Promise<FlowRow[]> {
  const url = new URL("rest/v1/flows_v2", withTrailingSlash(supabaseUrl));
  url.searchParams.set(
    "select",
    "name,uuid,user_id,updated_at,nodes,start_shared,start_unverified,is_public",
  );
  url.searchParams.set(
    "name",
    `in.(${REQUIRED_FIXTURE_FLOW_NAMES.map((name) => `"${name}"`).join(",")})`,
  );
  if (userId) {
    url.searchParams.set("user_id", `eq.${userId}`);
  }

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
  const rows = await response.json() as Array<FlowRow & { is_public?: boolean }>;
  return rows.map((row) => ({
    ...row,
    isPublic: row.isPublic ?? row.is_public,
  }));
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

function readMapStringField(value: unknown, key: string): string | undefined {
  if (!isRecord(value) || !isRecord(value.M)) {
    return undefined;
  }
  return readTaggedString(value.M[key]);
}

async function generateCurvePubkey(): Promise<string> {
  const keyPair = await crypto.subtle.generateKey(
    "Ed25519",
    true,
    ["sign", "verify"],
  ) as CryptoKeyPair;
  const raw = new Uint8Array(await crypto.subtle.exportKey("raw", keyPair.publicKey));
  return encodeBase58(raw);
}

function expectedOwnerPubkey(): string | undefined {
  const secret = resolveOptionalEnv("KEYPAIR", ["keypair", "OWNER_KEYPAIR"]);
  if (!secret) {
    return undefined;
  }
  const bytes = decodeBase58(secret);
  if (bytes.length < 64) {
    throw new Error(
      `expected KEYPAIR/OWNER_KEYPAIR to decode to at least 64 bytes, got ${bytes.length}`,
    );
  }
  return encodeBase58(bytes.slice(bytes.length - 32));
}

async function fetchWithTimeout(
  input: string | URL,
  init: RequestInit,
  label: string,
  timeoutMs = 30_000,
): Promise<Response> {
  try {
    return await fetch(input, {
      ...init,
      signal: AbortSignal.timeout(timeoutMs),
    });
  } catch (error) {
    throw new Error(
      `${label} failed: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

function apiKeyHeaders(apiKey: string): HeadersInit {
  return {
    "x-api-key": apiKey,
  };
}

async function fetchApiKeyUserId(serverUrl: string, apiKey: string): Promise<string | undefined> {
  const response = await fetchWithTimeout(
    buildUrl(serverUrl, "apikey/info"),
    {
      headers: apiKeyHeaders(apiKey),
    },
    "resolve APIKEY owner",
  );
  if (!response.ok) {
    return undefined;
  }
  const body = await response.json() as { user_id?: unknown };
  return typeof body.user_id === "string" ? body.user_id : undefined;
}

async function claimApiKeySession(
  serverUrl: string,
  apiKey: string,
): Promise<{ user_id: string; access_token: string }> {
  const response = await fetchWithTimeout(
    buildUrl(serverUrl, "auth/claim_token"),
    {
      method: "POST",
      headers: apiKeyHeaders(apiKey),
    },
    "claim APIKEY bearer session",
  );

  if (!response.ok) {
    throw new Error(
      `claim APIKEY bearer session failed: ${response.status} ${await response.text()}`,
    );
  }

  const body = await response.json() as {
    user_id?: unknown;
    access_token?: unknown;
  };
  if (typeof body.user_id !== "string" || typeof body.access_token !== "string") {
    throw new Error("claim APIKEY bearer session returned an unexpected response");
  }

  return {
    user_id: body.user_id,
    access_token: body.access_token,
  };
}

async function ensureOwnerSigningWallet(
  serverUrl: string,
  apiKey: string,
  userId: string,
): Promise<void> {
  const keypair = resolveOptionalEnv("KEYPAIR", ["keypair", "OWNER_KEYPAIR"]);
  const publicKey = expectedOwnerPubkey();
  if (!keypair || !publicKey) {
    return;
  }

  const session = await claimApiKeySession(serverUrl, apiKey);
  if (session.user_id !== userId) {
    throw new Error(
      `APIKEY bearer session resolved to ${session.user_id}, expected ${userId}`,
    );
  }

  const response = await fetchWithTimeout(
    buildUrl(serverUrl, "wallets/upsert"),
    {
      method: "POST",
      headers: {
        authorization: `Bearer ${session.access_token}`,
        "content-type": "application/json",
      },
      body: JSON.stringify({
        type: "HARDCODED",
        name: "e2e-owner-signing-wallet",
        public_key: publicKey,
        keypair,
        user_id: userId,
      }),
    },
    "upsert owner signing wallet",
  );

  if (!response.ok) {
    throw new Error(
      `upsert owner signing wallet failed: ${response.status} ${await response.text()}`,
    );
  }
}

async function fetchOwnerWalletTarget(
  supabaseUrl: string,
  serviceRoleKey: string,
  userId?: string,
): Promise<WalletTarget | undefined> {
  if (!userId) {
    return undefined;
  }

  const url = new URL("rest/v1/wallets", withTrailingSlash(supabaseUrl));
  url.searchParams.set("select", "id,public_key,purpose,type,created_at");
  url.searchParams.set("user_id", `eq.${userId}`);
  const preferredPubkey = expectedOwnerPubkey();
  url.searchParams.set("order", "created_at.desc");

  const response = await fetch(url, {
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
    },
  });

  if (!response.ok) {
    throw new Error(
      `failed to query owner wallets from ${url}: ${response.status} ${await response.text()}`,
    );
  }

  const rows = await response.json() as Array<{
    id?: number;
    public_key?: string;
    purpose?: string | null;
    type?: string | null;
    created_at?: string | null;
  }>;
  const preferred = rows.find((row) =>
    typeof row.id === "number" &&
    typeof row.public_key === "string" &&
    row.public_key === preferredPubkey
  ) ?? rows.find((row) =>
    typeof row.id === "number" &&
    typeof row.public_key === "string" &&
    row.purpose === "Main wallet"
  ) ?? rows.find((row) =>
    typeof row.id === "number" &&
    typeof row.public_key === "string"
  );

  if (!preferred || typeof preferred.id !== "number" || typeof preferred.public_key !== "string") {
    return undefined;
  }

  return {
    walletId: preferred.id,
    publicKey: preferred.public_key,
  };
}

function selectPreferredFixtureFlows(
  flows: FlowRow[],
  preferredUserId?: string,
): {
  selected: Map<string, FlowRow>;
  warnings: string[];
} {
  const grouped = new Map<string, FlowRow[]>();
  for (const flow of flows) {
    const rows = grouped.get(flow.name) ?? [];
    rows.push(flow);
    grouped.set(flow.name, rows);
  }

  const warnings: string[] = [];
  const selected = new Map<string, FlowRow>();
  for (const [name, rows] of grouped) {
    if (rows.length > 1) {
      warnings.push(
        `fixture flow "${name}" has ${rows.length} copies; preflight is using the preferred owner/latest row`,
      );
    }

    const sorted = [...rows].sort((a, b) => {
      const aPreferred = preferredUserId !== undefined && a.user_id === preferredUserId ? 1 : 0;
      const bPreferred = preferredUserId !== undefined && b.user_id === preferredUserId ? 1 : 0;
      if (aPreferred !== bPreferred) {
        return bPreferred - aPreferred;
      }
      const aTime = Date.parse(a.updated_at ?? "");
      const bTime = Date.parse(b.updated_at ?? "");
      const normalizedATime = Number.isNaN(aTime) ? 0 : aTime;
      const normalizedBTime = Number.isNaN(bTime) ? 0 : bTime;
      if (normalizedATime !== normalizedBTime) {
        return normalizedBTime - normalizedATime;
      }
      return a.uuid.localeCompare(b.uuid);
    });

    selected.set(name, sorted[0]);
  }

  if (preferredUserId !== undefined) {
    for (const [name, row] of selected) {
      if (row.user_id !== preferredUserId) {
        warnings.push(
          `fixture flow "${name}" is not owned by APIKEY user ${preferredUserId}; selected ${row.uuid} from user ${row.user_id ?? "unknown"}`,
        );
      }
    }
  }

  return { selected, warnings };
}

function collectFixtureIntegrityIssues(flows: FlowRow[]): {
  issues: string[];
  referencedFlowIds: string[];
} {
  const issues: string[] = [];
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
    const pubkey = await generateCurvePubkey();
    const startResponse = await fetchWithTimeout(
      buildUrl(serverUrl, `flow/start_unverified/${flowId}`),
      {
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
      },
      `start unverified fixture flow ${flowId}`,
    );

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

    const outputResponse = await fetchWithTimeout(
      buildUrl(serverUrl, `flow/output/${started.flow_run_id}`),
      {
        headers: {
          authorization: `Bearer ${started.token}`,
        },
      },
      `fetch unverified fixture output for ${started.flow_run_id}`,
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
  const response = await fetchWithTimeout(buildUrl(serverUrl, `flow/deploy/${flowId}`), {
    method: "POST",
    headers: apiKeyHeaders(apiKey),
  }, `deploy fixture flow ${flowId}`);

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

async function startOwnerFlow(
  serverUrl: string,
  apiKey: string,
  flowId: string,
  body: Record<string, unknown> = {},
): Promise<string> {
  const response = await fetchWithTimeout(
    buildUrl(serverUrl, `flow/start/${flowId}`),
    {
      method: "POST",
      headers: {
        ...apiKeyHeaders(apiKey),
        "content-type": "application/json",
      },
      body: JSON.stringify(body),
    },
    `start fixture flow ${flowId}`,
  );

  if (!response.ok) {
    throw new Error(
      `fixture flow start failed for ${flowId}: ${response.status} ${await response.text()}`,
    );
  }

  const bodyJson = await response.json() as { flow_run_id?: string };
  if (!bodyJson.flow_run_id) {
    throw new Error(
      `fixture flow start returned an unexpected response for ${flowId}`,
    );
  }
  return bodyJson.flow_run_id;
}

async function waitForApiInputUrl(
  serverUrl: string,
  apiKey: string,
  flowRunId: string,
  timeoutMs = 30_000,
): Promise<string> {
  return await new Promise((resolve, reject) => {
    const ws = new WebSocket(toWsUrl(serverUrl));
    let finished = false;
    let requestId = 0;
    let streamId: number | undefined;
    const timeoutId = setTimeout(() => {
      finish(new Error(`timed out waiting for ApiInput event for flow run ${flowRunId}`));
    }, timeoutMs);

    const finish = (result: string | Error) => {
      if (finished) {
        return;
      }
      finished = true;
      clearTimeout(timeoutId);
      try {
        ws.close();
      } catch {
        // Ignore close errors while surfacing the original result.
      }
      if (typeof result === "string") {
        resolve(result);
      } else {
        reject(result);
      }
    };

    ws.onopen = () => {
      ws.send(JSON.stringify({
        id: ++requestId,
        method: "Authenticate",
        params: { token: apiKey },
      }));
    };
    ws.onerror = () => {
      finish(new Error(`websocket error while waiting for ApiInput event for ${flowRunId}`));
    };
    ws.onclose = () => {
      if (!finished) {
        finish(new Error(`websocket closed before ApiInput event for ${flowRunId}`));
      }
    };
    ws.onmessage = (event) => {
      try {
        if (typeof event.data !== "string") {
          finish(new Error("received a non-text websocket message"));
          return;
        }
        const message = JSON.parse(event.data) as Record<string, unknown>;
        if (message.id === 1) {
          if (typeof message.Err === "string") {
            finish(new Error(`websocket authenticate failed: ${message.Err}`));
            return;
          }
          ws.send(JSON.stringify({
            id: ++requestId,
            method: "SubscribeFlowRunEvents",
            params: { flow_run_id: flowRunId },
          }));
          return;
        }
        if (message.id === 2) {
          if (typeof message.Err === "string") {
            finish(new Error(`websocket flow subscription failed: ${message.Err}`));
            return;
          }
          const candidate = isRecord(message.Ok) ? message.Ok.stream_id : undefined;
          if (typeof candidate !== "number") {
            finish(new Error("websocket flow subscription returned no stream_id"));
            return;
          }
          streamId = candidate;
          return;
        }
        if (typeof message.stream_id !== "number" || message.stream_id !== streamId) {
          return;
        }
        if (message.event === "ApiInput") {
          const data = isRecord(message.data) ? message.data : undefined;
          if (typeof data?.url !== "string") {
            finish(new Error(`ApiInput event for ${flowRunId} did not include a url`));
            return;
          }
          finish(normalizeApiInputUrl(data.url, serverUrl));
          return;
        }
        if (message.event === "NodeError" || message.event === "FlowError") {
          const data = isRecord(message.data) ? message.data : undefined;
          finish(new Error(
            `flow run ${flowRunId} failed before ApiInput submit: ${
              typeof data?.error === "string" ? data.error : JSON.stringify(data)
            }`,
          ));
        }
      } catch (error) {
        finish(error instanceof Error ? error : new Error(String(error)));
      }
    };
  });
}

async function fetchFlowOutputForApiKey(
  serverUrl: string,
  apiKey: string,
  flowRunId: string,
  timeoutMs = 30_000,
): Promise<unknown> {
  const response = await fetchWithTimeout(
    buildUrl(serverUrl, `flow/output/${flowRunId}`),
    {
      headers: apiKeyHeaders(apiKey),
    },
    `fetch flow output for ${flowRunId}`,
    timeoutMs,
  );

  if (!response.ok) {
    throw new Error(
      `flow output request failed for ${flowRunId}: ${response.status} ${await response.text()}`,
    );
  }

  return await response.json();
}

async function waitForOutputStringField(
  serverUrl: string,
  apiKey: string,
  flowRunId: string,
  key: string,
  expected: string,
  timeoutMs = 30_000,
): Promise<unknown> {
  const deadline = Date.now() + timeoutMs;
  let lastOutput: unknown = undefined;

  while (Date.now() < deadline) {
    lastOutput = await fetchFlowOutputForApiKey(serverUrl, apiKey, flowRunId, timeoutMs);
    if (readMapStringField(lastOutput, key) === expected) {
      return lastOutput;
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }

  throw new Error(
    `timed out waiting for flow output ${key}=${JSON.stringify(expected)} for ${flowRunId}; last output ${JSON.stringify(lastOutput)}`,
  );
}

async function waitForSignatureRequest(
  serverUrl: string,
  apiKey: string,
  flowRunId: string,
  timeoutMs = 30_000,
): Promise<{ pubkey?: string }> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const response = await fetchWithTimeout(
      buildUrl(serverUrl, `flow/signature_request/${flowRunId}`),
      {
        headers: apiKeyHeaders(apiKey),
      },
      `wait for signature request for ${flowRunId}`,
      timeoutMs,
    );

    if (response.ok) {
      return await response.json() as { pubkey?: string };
    }

    if (response.status !== 404) {
      throw new Error(
        `signature request fetch failed for ${flowRunId}: ${response.status} ${await response.text()}`,
      );
    }

    await new Promise((resolve) => setTimeout(resolve, 500));
  }

  throw new Error(`timed out waiting for signature request for ${flowRunId}`);
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

    const pubkey = await generateCurvePubkey();
    const startResponse = await fetchWithTimeout(
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
      `start anonymous deployment ${deploymentId}`,
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

    const outputResponse = await fetchWithTimeout(
      buildUrl(serverUrl, `flow/output/${started.flow_run_id}`),
      {
        headers: {
          authorization: `Bearer ${started.token}`,
        },
      },
      `fetch anonymous deployment output for ${started.flow_run_id}`,
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

async function probeApiInputSubmit(
  serverUrl: string,
  flowId: string,
): Promise<string | undefined> {
  const apiKey = Deno.env.get("APIKEY");
  if (!apiKey) {
    return "api-input submit probe skipped because APIKEY is not set";
  }

  try {
    const flowRunId = await startOwnerFlow(serverUrl, apiKey, flowId);
    const submitUrl = await waitForApiInputUrl(serverUrl, apiKey, flowRunId);
    const submitResponse = await fetchWithTimeout(
      submitUrl,
      {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify({
          value: { S: "hello" },
        }),
      },
      `submit api-input payload for ${flowRunId}`,
    );

    if (!submitResponse.ok) {
      return `api-input submit probe failed for flow run ${flowRunId}: ${submitResponse.status} ${await submitResponse.text()}`;
    }

    const output = await waitForOutputStringField(
      serverUrl,
      apiKey,
      flowRunId,
      "c",
      "hello",
    );
    const result = readMapStringField(output, "c");
    if (result !== "hello") {
      return `api-input submit probe returned unexpected output for ${flowRunId}: expected "hello", got ${JSON.stringify(output)}`;
    }
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }

  return undefined;
}

async function probeApiInputWebhook(
  serverUrl: string,
  flowId: string,
): Promise<string | undefined> {
  const apiKey = Deno.env.get("APIKEY");
  if (!apiKey) {
    return "api-input webhook probe skipped because APIKEY is not set";
  }

  const webhookUrl = configuredWebhookUrl(serverUrl);
  if (!webhookUrl) {
    console.warn(
      "Warning: api-input webhook probe skipped because FLOW_TEST_WEBHOOK_URL is not set " +
        "and FLOW_SERVER_URL is not loopback.",
    );
    return undefined;
  }

  try {
    const flowRunId = await startOwnerFlow(serverUrl, apiKey, flowId, {
      inputs: {
        webhook_url: { S: webhookUrl },
      },
    });
    const output = await waitForOutputStringField(
      serverUrl,
      apiKey,
      flowRunId,
      "c",
      "hello",
    );
    const result = readMapStringField(output, "c");
    if (result !== "hello") {
      return `api-input webhook probe returned unexpected output for ${flowRunId}: expected "hello" from ${webhookUrl}, got ${JSON.stringify(output)}.`;
    }
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
}

async function probeDeploymentSignatureRequestPubkey(
  serverUrl: string,
  supabaseUrl: string,
  serviceRoleKey: string,
  flowId: string,
): Promise<string | undefined> {
  const apiKey = Deno.env.get("APIKEY");
  if (!apiKey) {
    return "deployment signature-request probe skipped because APIKEY is not set";
  }

  const expectedPubkey = expectedOwnerPubkey();
  if (!expectedPubkey) {
    return "deployment signature-request probe skipped because KEYPAIR/OWNER_KEYPAIR is not set";
  }

  let deploymentId: string | undefined;
  try {
    deploymentId = await deployFixtureFlow(serverUrl, apiKey, flowId);
    await updateDeployment(supabaseUrl, serviceRoleKey, deploymentId, {
      start_permission: "Anonymous",
    });

    const starterPubkey = await generateCurvePubkey();
    const startResponse = await fetchWithTimeout(
      buildUrl(serverUrl, `deployment/start?id=${deploymentId}`),
      {
        method: "POST",
        headers: {
          authorization: `Bearer ${starterPubkey}`,
          "content-type": "application/json",
        },
        body: JSON.stringify({
          inputs: {
            sender: { B3: starterPubkey },
            n: { U: "2" },
          },
        }),
      },
      `start deployment ${deploymentId} for signature-request probe`,
    );

    if (!startResponse.ok) {
      return `deployment signature-request probe failed to start deployment ${deploymentId}: ${startResponse.status} ${await startResponse.text()}`;
    }

    const started = await startResponse.json() as { flow_run_id?: string };
    if (!started.flow_run_id) {
      return `deployment signature-request probe received an unexpected start response for deployment ${deploymentId}`;
    }

    const request = await waitForSignatureRequest(
      serverUrl,
      apiKey,
      started.flow_run_id,
    );
    if (typeof request.pubkey !== "string" || request.pubkey.length === 0) {
      return "deployment signature-request probe returned no pubkey";
    }
    if (request.pubkey !== expectedPubkey) {
      console.warn(
        `Warning: deployment signature-request probe saw ${request.pubkey} instead of owner pubkey ${expectedPubkey}. ` +
          "Continuing because the probe only requires that signature requests are emitted.",
      );
    }
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  } finally {
    if (deploymentId) {
      await deleteDeployment(supabaseUrl, serviceRoleKey, deploymentId).catch((error) => {
        console.warn(
          `Warning: could not clean up signature-request probe deployment ${deploymentId}: ${
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
  const apiKey = Deno.env.get("APIKEY");
  const preferredUserId = apiKey
    ? await fetchApiKeyUserId(serverUrl, apiKey).catch(() => undefined)
    : undefined;
  const flows = await fetchDetailedFixtureFlows(
    supabaseUrl,
    serviceRoleKey,
    preferredUserId,
  );
  const { selected, warnings } = selectPreferredFixtureFlows(flows, preferredUserId);
  for (const warning of warnings) {
    console.warn(`Warning: ${warning}`);
  }

  const selectedFlows = Array.from(selected.values());
  const { issues, referencedFlowIds } = collectFixtureIntegrityIssues(selectedFlows);
  const missingReferencedFlowIds = await verifyReferencedFlowsExist(
    supabaseUrl,
    serviceRoleKey,
    referencedFlowIds,
  );
  for (const missingId of missingReferencedFlowIds) {
    issues.push(`fixture interflow reference missing target flow ${missingId}`);
  }

  const addFlow = selected.get("Add");
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

  const apiInputFlow = selected.get("API Input");
  if (!apiInputFlow) {
    issues.push('fixture preflight could not find the "API Input" flow for api-input probes');
  } else {
    const submitIssue = await probeApiInputSubmit(serverUrl, apiInputFlow.uuid);
    if (submitIssue) {
      issues.push(`api-input submit probe failed: ${submitIssue}`);
    }
    const webhookIssue = await probeApiInputWebhook(serverUrl, apiInputFlow.uuid);
    if (webhookIssue) {
      issues.push(`api-input webhook probe failed: ${webhookIssue}`);
    }
  }

  const deployRunFlow = selected.get("Transfer SOL");
  if (!deployRunFlow) {
    issues.push('fixture preflight could not find the "Transfer SOL" flow for the deployment signature-request probe');
  } else {
    const signatureIssue = await probeDeploymentSignatureRequestPubkey(
      serverUrl,
      supabaseUrl,
      serviceRoleKey,
      deployRunFlow.uuid,
    );
    if (signatureIssue) {
      issues.push(`deployment signature-request probe failed: ${signatureIssue}`);
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

async function listMissingPreferredFixtureFlows(
  supabaseUrl: string,
  serviceRoleKey: string,
  preferredUserId?: string,
): Promise<string[]> {
  if (!preferredUserId) {
    return await listMissingFixtureFlows(supabaseUrl, serviceRoleKey);
  }

  const rows = await fetchDetailedFixtureFlows(
    supabaseUrl,
    serviceRoleKey,
    preferredUserId,
  );
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

async function provisionLocalApiKey(
  supabaseUrl: string,
  serviceRoleKey: string,
  userId: string,
): Promise<string> {
  const { fullKey, keyHash, trimmedKey } = generateApiKeyMaterial();
  const baseUrl = withTrailingSlash(supabaseUrl);

  const deleteUrl = new URL("rest/v1/apikeys", baseUrl);
  deleteUrl.searchParams.set("user_id", `eq.${userId}`);
  deleteUrl.searchParams.set("name", `eq.${LOCAL_API_KEY_NAME}`);
  const deleteResponse = await fetch(deleteUrl, {
    method: "DELETE",
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
      prefer: "return=minimal",
    },
  });
  if (!deleteResponse.ok) {
    throw new Error(
      `local APIKEY cleanup failed: ${deleteResponse.status} ${await deleteResponse.text()}`,
    );
  }

  const insertResponse = await fetch(new URL("rest/v1/apikeys", baseUrl), {
    method: "POST",
    headers: {
      apikey: serviceRoleKey,
      authorization: `Bearer ${serviceRoleKey}`,
      "content-type": "application/json",
      prefer: "return=minimal",
    },
    body: JSON.stringify({
      key_hash: keyHash,
      user_id: userId,
      name: LOCAL_API_KEY_NAME,
      trimmed_key: trimmedKey,
      created_at: new Date().toISOString(),
    }),
  });
  if (!insertResponse.ok) {
    throw new Error(
      `local APIKEY insert failed: ${insertResponse.status} ${await insertResponse.text()}`,
    );
  }

  return fullKey;
}

async function writeLocalTestEnv(path: string, values: Record<string, string>): Promise<void> {
  const lines = [
    "# Generated by docker/bootstrap-test-fixtures.ts for local e2e runs.",
    ...Object.entries(values).map(([key, value]) => `${key}=${JSON.stringify(value)}`),
    "",
  ];
  await Deno.writeTextFile(path, lines.join("\n"));
}

async function ensureUsableApiKey(
  serverUrl: string,
  supabaseUrl: string,
  serviceRoleKey: string,
  expectedUserId?: string,
  localEnvPath?: string,
): Promise<string | undefined> {
  const configuredApiKey = Deno.env.get("APIKEY");
  const configuredUserId = configuredApiKey
    ? await fetchApiKeyUserId(serverUrl, configuredApiKey).catch(() => undefined)
    : undefined;

  if (configuredApiKey && (!expectedUserId || configuredUserId === expectedUserId)) {
    return configuredApiKey;
  }

  if (
    !expectedUserId ||
    !isLoopbackUrl(serverUrl) ||
    !isLoopbackUrl(supabaseUrl)
  ) {
    return configuredApiKey;
  }

  const localApiKey = await provisionLocalApiKey(supabaseUrl, serviceRoleKey, expectedUserId);
  Deno.env.set("APIKEY", localApiKey);
  if (localEnvPath) {
    await writeLocalTestEnv(localEnvPath, { APIKEY: localApiKey });
  }
  console.log(`Provisioned a local APIKEY for fixture owner ${expectedUserId}.`);
  return localApiKey;
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
  const args = parseArgs(Deno.args, {
    boolean: ["force", "verify", "preflight-only"],
    string: ["file", "server", "supabase-url"],
    default: {
      verify: true,
      "preflight-only": false,
    },
  });

  const dockerDir = dirname(fromFileUrl(import.meta.url));
  const repoEnv = join(dockerDir, "..", ".env");
  const dockerEnv = join(dockerDir, ".env");
  const localTestEnv = join(dockerDir, LOCAL_TEST_ENV_FILE);
  // Load docker-local .env first so its keys (local Supabase JWTs) take
  // priority over the hosted keys in the repo-root .env.  @std/dotenv
  // load() with export:true never overwrites existing env vars, so
  // whichever file is loaded first wins.
  await load({ export: true, envPath: localTestEnv }).catch(() => undefined);
  await load({ export: true, envPath: dockerEnv }).catch(() => undefined);
  await load({ export: true, envPath: repoEnv }).catch(() => undefined);
  await load({ export: true }).catch(() => undefined);

  const file = args.file ?? join(dockerDir, "export.json");
  const serverUrl = args.server ?? Deno.env.get("FLOW_SERVER_URL") ??
    "http://127.0.0.1:8080";
  const supabaseUrl = args["supabase-url"] ?? Deno.env.get("SUPABASE_URL") ??
    "http://127.0.0.1:8000";
  const serviceRoleKey = getEnv("SERVICE_ROLE_KEY");
  const rawData = await readFixtureFile(file) as FixtureExport;
  let apiKey = Deno.env.get("APIKEY");
  const apiKeyUserId = apiKey
    ? await fetchApiKeyUserId(serverUrl, apiKey).catch(() => undefined)
    : undefined;
  if (apiKey && apiKeyUserId) {
    await ensureOwnerSigningWallet(serverUrl, apiKey, apiKeyUserId).catch((error) => {
      console.warn(
        `Warning: could not upsert the owner signing wallet before fixture bootstrap. ${
          error instanceof Error ? error.message : String(error)
        }`,
      );
    });
  }
  let walletTarget = await fetchOwnerWalletTarget(
    supabaseUrl,
    serviceRoleKey,
    apiKeyUserId,
  ).catch(() => undefined);
  let prepared = prepareFixtureImportData(rawData, apiKeyUserId, walletTarget);
  let data = prepared.data;
  let expectedUserId = prepared.importedUserId;

  if (!args.force) {
    const missing = await listMissingPreferredFixtureFlows(
      supabaseUrl,
      serviceRoleKey,
      expectedUserId,
    );
    let refreshingExistingFixtures = false;
    if (missing.length === 0) {
      console.log("Local test fixtures already present.");
      if (!args.verify) {
        await verifyApiKey(serverUrl, expectedUserId);
        return;
      }

      try {
        apiKey = await ensureUsableApiKey(
          serverUrl,
          supabaseUrl,
          serviceRoleKey,
          expectedUserId,
          localTestEnv,
        );
        if (apiKey && expectedUserId) {
          await ensureOwnerSigningWallet(serverUrl, apiKey, expectedUserId).catch((error) => {
            console.warn(
              `Warning: could not upsert the owner signing wallet for the local APIKEY. ${
                error instanceof Error ? error.message : String(error)
              }`,
            );
          });
        }
        await runFixturePreflight(serverUrl, supabaseUrl, serviceRoleKey);
        await verifyApiKey(serverUrl, expectedUserId);
        return;
      } catch (error) {
        if (args["preflight-only"]) {
          throw error;
        }
        console.warn(
          `Warning: fixture preflight failed against the existing dataset. ` +
            `Bootstrapping a fresh copy to repair stale fixtures. ${
              error instanceof Error ? error.message : String(error)
            }`,
        );
        refreshingExistingFixtures = true;
      }
    }

    if (args["preflight-only"]) {
      throw new Error(
        `fixture preflight requested, but local fixture flows are missing: ${missing.join(", ")}. ` +
          "Run the bootstrap without --preflight-only first.",
      );
    }

    if (refreshingExistingFixtures) {
      console.log("Refreshing fixture flows for the current APIKEY owner.");
    } else {
      console.log(`Missing fixture flows: ${missing.join(", ")}`);
    }
  }

  if (args["preflight-only"]) {
    console.log("Running fixture preflight without importing data.");
    if (args.verify) {
      apiKey = await ensureUsableApiKey(
        serverUrl,
        supabaseUrl,
        serviceRoleKey,
        expectedUserId,
        localTestEnv,
      );
      if (apiKey && expectedUserId) {
        await ensureOwnerSigningWallet(serverUrl, apiKey, expectedUserId).catch((error) => {
          console.warn(
            `Warning: could not upsert the owner signing wallet for the local APIKEY. ${
              error instanceof Error ? error.message : String(error)
            }`,
          );
        });
      }
      await runFixturePreflight(serverUrl, supabaseUrl, serviceRoleKey);
    }
    await verifyApiKey(serverUrl, expectedUserId);
    return;
  }

  console.log(`Reading fixture export from ${file}`);
  const effectiveApiKeyUserId = apiKey
    ? await fetchApiKeyUserId(serverUrl, apiKey).catch(() => undefined)
    : undefined;
  if (effectiveApiKeyUserId && effectiveApiKeyUserId !== apiKeyUserId) {
    walletTarget = await fetchOwnerWalletTarget(
      supabaseUrl,
      serviceRoleKey,
      effectiveApiKeyUserId,
    ).catch(() => walletTarget);
    prepared = prepareFixtureImportData(rawData, effectiveApiKeyUserId, walletTarget);
    data = prepared.data;
    expectedUserId = prepared.importedUserId;
  }
  for (const note of prepared.notes) {
    console.log(`Fixture import note: ${note}`);
  }

  console.log(`Importing fixtures into ${serverUrl}`);
  try {
    await importFixtureData(serverUrl, serviceRoleKey, data);
  } catch (error) {
    if (
      typeof data.flows === "string" &&
      (
        shouldUseRestImportFallback(error) ||
        prepared.notes.includes(
          "importing flow data only to avoid auth/api-key conflicts with an existing local user",
        )
      )
    ) {
      console.warn(
        `Warning: bulk fixture import is unavailable in this environment. ` +
          `Falling back to direct flows_v2 REST repair. ${
            error instanceof Error ? error.message : String(error)
          }`,
      );
      await importFixtureFlowsViaRest(supabaseUrl, serviceRoleKey, data.flows);
    } else {
      throw error;
    }
  }

  if (args.verify) {
    const missing = await listMissingPreferredFixtureFlows(
      supabaseUrl,
      serviceRoleKey,
      expectedUserId,
    );
    if (missing.length > 0) {
      throw new Error(
        `fixture bootstrap finished, but flows are still missing: ${missing.join(", ")}`,
      );
    }
  }

  const rows = await fetchFixtureFlows(supabaseUrl, serviceRoleKey, expectedUserId);
  console.log("Fixture flows available:");
  for (const row of rows.sort((a, b) => a.name.localeCompare(b.name))) {
    console.log(`- ${row.name}: ${row.uuid}`);
  }

  if (args.verify) {
    apiKey = await ensureUsableApiKey(
      serverUrl,
      supabaseUrl,
      serviceRoleKey,
      expectedUserId,
      localTestEnv,
    );
    if (apiKey && expectedUserId) {
      await ensureOwnerSigningWallet(serverUrl, apiKey, expectedUserId).catch((error) => {
        console.warn(
          `Warning: could not upsert the owner signing wallet for the local APIKEY. ${
            error instanceof Error ? error.message : String(error)
          }`,
        );
      });
    }
    await runFixturePreflight(serverUrl, supabaseUrl, serviceRoleKey);
  }

  await verifyApiKey(serverUrl, expectedUserId);
}

await main();
