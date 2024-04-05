import { start } from "jsr:@space-operator/deno-command-rpc@0.7.0";
// import { start } from "./@space-operator/deno-command-rpc/src/mod.ts";
import UserCommand from "./__cmd.ts";

start(new UserCommand(), { hostname: "127.0.0.1", port: 0 });
