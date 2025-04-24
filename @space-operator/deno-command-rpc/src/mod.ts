import { PromiseSet, CaptureLog } from "./utils.ts";
import {
  Context,
  type IValue,
  type ContextProxy,
  Value,
  type CommandTrait,
  Application,
  type ListenOptions,
  Router,
  Status,
} from "./deps.ts";

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

interface Response<T> {
  envelope: string;
  success: boolean;
  data: T;
}

export async function start(
  cmd: CommandTrait,
  listenOptions: Pick<ListenOptions, "hostname" | "port">
) {
  const realConsole = globalThis.console;
  const router = new Router();
  router.post("/call", async (ctx) => {
    const req: IRequest<any> = await ctx.request.body.json();
    if (req.svc_name === RUN_SVC) {
      const input: RunInput = req.input;
      const params = Value.fromJSON({ M: input.params });
      let data: RunOutput;
      let success = false;
      const logPromises = new PromiseSet();
      try {
        // build context
        const context = new Context(input.ctx);

        // replace console
        if (context.command?.log) {
          globalThis.console = new CaptureLog(
            realConsole,
            context.command?.log,
            logPromises
          );
        } else {
          globalThis.console = realConsole;
        }

        // deserialize inputs
        const convertedInputs =
          typeof cmd.deserializeInputs === "function"
            ? cmd.deserializeInputs(params.M!)
            : params.toJSObject();

        // run command
        const outputs = await cmd.run(context, convertedInputs);

        // serialize outputs
        const convertedOutputs: Record<string, Value> =
          typeof cmd.serializeOutputs === "function"
            ? cmd.serializeOutputs(outputs)
            : new Value(outputs).M!;

        data = { Ok: convertedOutputs };
        success = true;
      } catch (error: any) {
        data = { Err: error.toString() };
        success = false;
      }

      // wait for all logs to be inserted before responding
      // because the ServiceProxy will be dropped then.
      await logPromises.wait();

      const resp: Response<RunOutput> = {
        envelope: req.envelope,
        success,
        data,
      };
      ctx.response.body = resp;
      ctx.response.type = "application/json";
      ctx.response.status = Status.OK;
    } else {
      ctx.response.body = {
        envelope: req.envelope,
        success: false,
        data: "not found",
      };
      ctx.response.type = "application/json";
      ctx.response.status = Status.OK;
    }
  });
  const app = new Application();
  app.addEventListener("listen", (ev) => {
    Deno.stdout.writeSync(new TextEncoder().encode(ev.port.toString() + "\n"));
  });
  app.use(router.routes());
  app.use(router.allowedMethods());
  await app.listen({
    hostname: listenOptions.hostname,
    port: listenOptions.port,
  });
}
