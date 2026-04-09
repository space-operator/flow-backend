/**
 * Bun-native command RPC server.
 *
 * Same HTTP protocol as @space-operator/deno-command-rpc but uses
 * Bun.serve() instead of Oak. Drop-in replacement for compiled nodes.
 */

import { createServer } from "node:net";
import { PromiseSet, CaptureLog } from "./utils.ts";
import {
  Context,
  type IValue,
  type ContextProxy,
  Value,
  type CommandTrait,
} from "@space-operator/flow-lib-bun";

const RUN_SVC = "run";

interface IRequest<T> {
  envelope: string;
  svc_name: string;
  svc_id: string;
  input: T;
}

interface RunInput {
  ctx: ContextProxy;
  params: Record<string, IValue>;
}

interface RunOutput {
  Ok?: Record<string, IValue>;
  Err?: string;
}

interface IResponse<T> {
  envelope: string;
  success: boolean;
  data: T;
}

async function resolveListenPort(port: number): Promise<number> {
  if (port !== 0) {
    return port;
  }

  return await new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close();
        reject(new Error("failed to reserve a Bun RPC port"));
        return;
      }

      server.close((error) => {
        if (error) {
          reject(error);
          return;
        }
        resolve(address.port);
      });
    });
  });
}

export async function start(
  cmd: CommandTrait,
  listenOptions: { hostname: string; port: number }
) {
  const realConsole = globalThis.console;
  const port = await resolveListenPort(listenOptions.port);

  const server = Bun.serve({
    hostname: listenOptions.hostname,
    port,
    async fetch(request: Request): Promise<Response> {
      const url = new URL(request.url);

      if (request.method === "POST" && url.pathname === "/call") {
        const req: IRequest<any> = await request.json();

        if (req.svc_name === RUN_SVC) {
          const input: RunInput = req.input;
          const params = Value.fromJSON({ M: input.params });
          let data: RunOutput;
          let success = false;
          const logPromises = new PromiseSet();

          try {
            const context = new Context(input.ctx);

            // Replace console for log capture
            if (context.command?.log) {
              globalThis.console = new CaptureLog(
                realConsole,
                context.command.log,
                logPromises
              );
            } else {
              globalThis.console = realConsole;
            }

            // Deserialize inputs
            const convertedInputs =
              typeof cmd.deserializeInputs === "function"
                ? cmd.deserializeInputs(params.M!)
                : params.toJSObject();

            // Run command
            const outputs = await cmd.run(context, convertedInputs);

            // Serialize outputs
            const convertedOutputs: Record<string, Value> =
              typeof cmd.serializeOutputs === "function"
                ? cmd.serializeOutputs(outputs)
                : new Value(outputs).M!;

            data = { Ok: convertedOutputs };
            success = true;
          } catch (error: any) {
            data = { Err: error.toString() };
            success = false;
          } finally {
            await logPromises.wait();
            globalThis.console = realConsole;
          }

          const resp: IResponse<RunOutput> = {
            envelope: req.envelope,
            success,
            data,
          };

          return new Response(JSON.stringify(resp), {
            headers: { "content-type": "application/json" },
          });
        }

        return new Response(
          JSON.stringify({
            envelope: req.envelope,
            success: false,
            data: "not found",
          }),
          { headers: { "content-type": "application/json" } }
        );
      }

      return new Response("not found", { status: 404 });
    },
  });

  // Print port to stdout — same protocol as Deno version
  // The Rust backend reads the first line to get the port
  process.stdout.write(server.port.toString() + "\n");
}
