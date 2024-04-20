import { rpc } from "./deps.ts";
import NODE_DATA from "./node-data.json" with { type: "json" };
import UserCommand from "./cmd.ts";

rpc.start(new UserCommand(NODE_DATA), { hostname: "127.0.0.1", port: 0 });
