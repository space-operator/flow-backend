// import { start } from "jsr:@space-operator/deno-command-rpc@0.9.4";
import { start } from "./@space-operator/deno-command-rpc/src/mod.ts";
import UserCommand from "./__cmd.ts";

start(new UserCommand(__NODE_DATA), { hostname: "127.0.0.1", port: 0 });
