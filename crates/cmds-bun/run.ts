/**
 * Bun run harness — equivalent to cmds-deno/run.ts
 *
 * Reads node-data.json and cmd.ts from the same directory,
 * starts the Bun RPC server on a random port, prints the port to stdout.
 */

function formatProcessError(event: string, error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  const stack = error instanceof Error ? error.stack ?? "" : "";
  return `[bun-rpc] ${event}: ${message}\n${stack}\n`;
}

process.on("unhandledRejection", (reason: unknown) => {
  process.stderr.write(formatProcessError("unhandledRejection", reason));
});

process.on("uncaughtException", (error: unknown) => {
  process.stderr.write(formatProcessError("uncaughtException", error));
  process.exit(1);
});

import { start } from "@space-operator/bun-command-rpc";
import NODE_DATA from "./node-data.json";
import UserCommand from "./cmd.ts";

start(new UserCommand(NODE_DATA), { hostname: "127.0.0.1", port: 0 });
