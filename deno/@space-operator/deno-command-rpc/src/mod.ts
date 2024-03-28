import { type Context, type IValue, Value } from "@space-operator/flow-lib";
import { Application, Router, Status } from "@oak/oak";

export const RUN_SVC = "run";

export interface CommandTrait {
  run(
    ctx: Context,
    params: Record<string, Value>
  ): Promise<Record<string, Value>>;
}

export interface IRequest<T> {
  envelope: string;
  svc_name: string;
  svc_id: string;
  input: T;
}

export interface RunInput {
  ctx: Context;
  params: Record<string, IValue>;
}

export interface RunOutput {
  Ok?: Record<string, IValue>;
  Err?: string;
}

export interface Response<T> {
  envelope: string;
  success: boolean;
  data: T;
}

export async function start(cmd: CommandTrait) {
  const router = new Router();
  router.post("/call", async (ctx) => {
    const req: IRequest<any> = await ctx.request.body.json();
    console.log(req);
    if (req.svc_name === RUN_SVC) {
      const input = req.input as RunInput;
      const params = Value.fromJSON({ M: input.params }).M!;
      let data: RunOutput;
      let success = false;
      try {
        data = { Ok: await cmd.run(input.ctx, params) };
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
  await app.listen({ port: 0 });
}
