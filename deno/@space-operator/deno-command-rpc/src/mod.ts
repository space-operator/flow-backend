import {
  type ServiceProxy,
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

type Console = typeof globalThis.console;
type Level = "INFO" | "ERROR" | "WARN" | "DEBUG" | "TRACE";

class CaptureLog implements Console {
  #original: Console;
  #service: ServiceProxy;
  #promises: Promise<void>[];
  constructor(
    original: Console,
    service: ServiceProxy,
    promises: Promise<void>[]
  ) {
    this.#original = original;
    this.#service = service;
    this.#promises = promises;
  }

  #formatLogContent(data: any[]): string {
    let str = "";
    data.forEach((value, index) => {
      if (index > 0) str += " ";
      switch (typeof value) {
        case "string":
          str += value;
          break;
        case "number":
          str += value.toString();
          break;
        case "bigint":
          str += value.toString();
          break;
        case "boolean":
          str += value.toString();
          break;
        case "symbol":
          str += value.description;
          break;
        case "undefined":
          str += "undefined";
          break;
        case "object":
          str += JSON.stringify(value);
          break;
        case "function":
          str += "[Function]";
          break;
      }
    });
    return str;
  }

  #call(level: Level, data: any[]) {
    const promise = fetch(new URL("call", this.#service.base_url), {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify({
        envelope: "",
        svc_name: this.#service.name,
        svc_id: this.#service.id,
        input: {
          level,
          content: this.#formatLogContent(data),
        },
      }),
    }).then(() => {});
    this.#promises.push(promise);
  }

  clear(): void {}
  debug(...data: any[]): void {
    this.#call("DEBUG", data);
  }
  error(...data: any[]): void {
    this.#call("ERROR", data);
  }
  info(...data: any[]): void {
    this.#call("INFO", data);
  }
  log(...data: any[]): void {
    this.#call("INFO", data);
  }
  trace(...data: any[]): void {
    this.#call("TRACE", data);
  }
  warn(...data: any[]): void {
    this.#call("WARN", data);
  }

  assert(condition?: boolean | undefined, ...data: any[]): void {
    return this.#original.assert(condition, ...data);
  }

  count(label?: string | undefined): void {
    return this.#original.count(label);
  }
  countReset(label?: string | undefined): void {
    return this.#original.countReset(label);
  }
  dir(item?: any, options?: any): void {
    return this.#original.dir(item, options);
  }
  dirxml(...data: any[]): void {
    return this.#original.dirxml(...data);
  }
  group(...data: any[]): void {
    return this.#original.group(...data);
  }
  groupCollapsed(...data: any[]): void {
    return this.#original.groupCollapsed(...data);
  }
  groupEnd(): void {
    return this.#original.groupEnd();
  }
  profile(label?: string | undefined): void {
    return this.#original.profile(label);
  }
  profileEnd(label?: string | undefined): void {
    return this.#original.profileEnd(label);
  }
  table(tabularData?: any, properties?: string[] | undefined): void {
    return this.#original.table(tabularData, properties);
  }
  time(label?: string | undefined): void {
    return this.#original.time(label);
  }
  timeEnd(label?: string | undefined): void {
    return this.#original.timeEnd(label);
  }
  timeLog(label?: string | undefined, ...data: any[]): void {
    return this.#original.timeLog(label, ...data);
  }
  timeStamp(label?: string | undefined): void {
    return this.#original.timeStamp(label);
  }
}

export async function start(
  cmd: CommandTrait,
  listenOptions: Pick<ListenOptions, "hostname" | "port">
) {
  const originalConsole = globalThis.console;
  const router = new Router();
  router.post("/call", async (ctx) => {
    const req: IRequest<any> = await ctx.request.body.json();
    if (req.svc_name === RUN_SVC) {
      const input = req.input as RunInput;
      const params = Value.fromJSON({ M: input.params });
      const jsParams = params.toJSObject();
      let data: RunOutput;
      let success = false;
      const logPromises: Promise<void>[] = [];
      try {
        const context = new Context(input.ctx);
        if (context.command?.log) {
          globalThis.console = new CaptureLog(
            originalConsole,
            context.command?.log,
            logPromises
          );
        } else {
          globalThis.console = originalConsole;
        }
        data = { Ok: new Value(await cmd.run(context, jsParams)).M! };
        success = true;
      } catch (error) {
        data = { Err: error.toString() };
        success = false;
      }
      await Promise.allSettled(logPromises);
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
