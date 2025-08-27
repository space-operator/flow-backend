#!/usr/bin/env -S deno run --allow-read --allow-write

import {
  encodeBase58,
  encodeBase64,
  encodeBase64Url,
  encodeHex,
} from "@std/encoding";
import { load } from "@std/dotenv";
import { crypto } from "@std/crypto/crypto";
import * as toml from "@std/toml";
import * as fs from "@std/fs";

const ENV_PATH = ".env";
const CONFIG_PATH = ".config.toml";
const ENV_TEMPLATE = "env.example";
const CONFIG_TEMPLATE = "flow-server-config.toml";

async function initHmac(secret: string): Promise<CryptoKey> {
  return await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    {
      name: "HMAC",
      hash: "SHA-256",
    },
    false,
    ["sign"],
  );
}

async function generateKey(secret: CryptoKey, role: string): Promise<string> {
  const now = Math.floor(new Date().getTime() / 1000);
  const payload = JSON.stringify({
    role,
    iss: "supabase",
    iat: now,
    exp: now + 5 * 365 * 3600 * 24, // 5 years
  });
  const headers = JSON.stringify({
    alg: "HS256",
    typ: "JWT",
  });
  const data = `${encodeBase64Url(headers)}.${encodeBase64Url(payload)}`;
  const signature = await crypto.subtle.sign(
    {
      name: "HMAC",
      hash: "SHA-256",
    },
    secret,
    new TextEncoder().encode(data),
  );
  return `${data}.${encodeBase64Url(signature)}`;
}

const encryptionKey = encodeBase64(crypto.getRandomValues(new Uint8Array(32)));

const jwtSecret = encodeBase64(crypto.getRandomValues(new Uint8Array(64)));

const hmacKey = await initHmac(jwtSecret);
const anonKey = await generateKey(hmacKey, "anon");
const serviceRoleKey = await generateKey(hmacKey, "service_role");

const postgresPassword = encodeBase58(
  crypto.getRandomValues(new Uint8Array(15)),
);
const flowRunnerPassword = encodeBase58(
  crypto.getRandomValues(new Uint8Array(15)),
);
const dashboardPassword = encodeBase58(
  crypto.getRandomValues(new Uint8Array(8)),
);

const irohSecretKey = encodeHex(crypto.getRandomValues(new Uint8Array(32)));

const env = await load({ envPath: ENV_TEMPLATE });
env["POSTGRES_PASSWORD"] = postgresPassword;
env["JWT_SECRET"] = jwtSecret;
env["ANON_KEY"] = anonKey;
env["SERVICE_ROLE_KEY"] = serviceRoleKey;
env["DASHBOARD_PASSWORD"] = dashboardPassword;
env["FLOW_RUNNER_PASSWORD"] = flowRunnerPassword;
env["ENCRYPTION_KEY"] = encryptionKey;
env["IROH_SECRET_KEY"] = irohSecretKey;
const envContent = Object.entries(env)
  .map(([k, v]) => `${k}=${JSON.stringify(v)}`)
  .join("\n") + "\n";

// deno-lint-ignore no-explicit-any
const config: any = toml.parse(await Deno.readTextFile(CONFIG_TEMPLATE));
config.supabase.jwt_key = jwtSecret;
config.supabase.service_key = serviceRoleKey;
config.supabase.anon_key = anonKey;
config.db.password = flowRunnerPassword;
config.db.encryption_key = encryptionKey;
config.iroh.secret_key = irohSecretKey;
const configContent = toml.stringify(config) + "\n";

const fileExists: string[] = [];
if (await fs.exists(ENV_PATH)) fileExists.push(ENV_PATH);
if (await fs.exists(CONFIG_PATH)) fileExists.push(CONFIG_PATH);
if (fileExists.length > 0) {
  console.log("Secret files already exist, please remove them before running:");
  for (const path of fileExists) {
    console.log(`\t${path}`);
  }
  Deno.exit(1);
}

console.log("Writing", ENV_PATH);
await Deno.writeTextFile(ENV_PATH, envContent);

console.log("Writing", CONFIG_PATH);
await Deno.writeTextFile(CONFIG_PATH, configContent);
