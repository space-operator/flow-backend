import {
  Context,
  type IValue,
  Value,
  type ContextData,
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
  ctx: ContextData;
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
  const router = new Router();
  router.post("/call", async (ctx) => {
    const req: IRequest<any> = await ctx.request.body.json();
    if (req.svc_name === RUN_SVC) {
      const input = req.input as RunInput;
      const params = Value.fromJSON({ M: input.params });
      const jsParams = params.toJSObject();
      let data: RunOutput;
      let success = false;
      const context = new Context(input.ctx);
      try {
        data = { Ok: new Value(await cmd.run(context, jsParams)).M! };
        success = true;
      } catch (error) {
        data = { Err: error.toString() };
        success = false;
      }
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
