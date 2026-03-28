/**
 * Bun run harness — equivalent to cmds-deno/run.ts
 *
 * Reads node-data.json and cmd.ts from the same directory,
 * starts the Bun RPC server on a random port, prints the port to stdout.
 */
import { start } from "@space-operator/bun-command-rpc";
import NODE_DATA from "./node-data.json";
import UserCommand from "./cmd.ts";

start(new UserCommand(NODE_DATA), { hostname: "127.0.0.1", port: 0 });
