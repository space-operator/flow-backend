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

class PromiseSet {
  #counter: bigint = 0n;
  #promises: Map<bigint, Promise<void>> = new Map();

  push(p: Promise<any>) {
    const id = this.#counter;
    this.#counter += 1n;
    this.#promises.set(
      id,
      p.then(() => {
        this.#promises.delete(id);
      })
    );
  }

  async wait(): Promise<void> {
    const promises = [...this.#promises.values()];
    this.#promises = new Map();
    await Promise.allSettled(promises);
  }
}

class CaptureLog implements Console {
  #realConsole: Console;
  #service: ServiceProxy;
  #promises: PromiseSet;
  constructor(original: Console, service: ServiceProxy, promises: PromiseSet) {
    this.#realConsole = original;
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
    });
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
    return this.#realConsole.assert(condition, ...data);
  }

  count(label?: string | undefined): void {
    return this.#realConsole.count(label);
  }
  countReset(label?: string | undefined): void {
    return this.#realConsole.countReset(label);
  }
  dir(item?: any, options?: any): void {
    return this.#realConsole.dir(item, options);
  }
  dirxml(...data: any[]): void {
    return this.#realConsole.dirxml(...data);
  }
  group(...data: any[]): void {
    return this.#realConsole.group(...data);
  }
  groupCollapsed(...data: any[]): void {
    return this.#realConsole.groupCollapsed(...data);
  }
  groupEnd(): void {
    return this.#realConsole.groupEnd();
  }
  profile(label?: string | undefined): void {
    return this.#realConsole.profile(label);
  }
  profileEnd(label?: string | undefined): void {
    return this.#realConsole.profileEnd(label);
  }
  table(tabularData?: any, properties?: string[] | undefined): void {
    return this.#realConsole.table(tabularData, properties);
  }
  time(label?: string | undefined): void {
    return this.#realConsole.time(label);
  }
  timeEnd(label?: string | undefined): void {
    return this.#realConsole.timeEnd(label);
  }
  timeLog(label?: string | undefined, ...data: any[]): void {
    return this.#realConsole.timeLog(label, ...data);
  }
  timeStamp(label?: string | undefined): void {
    return this.#realConsole.timeStamp(label);
  }
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
      const jsParams = params.toJSObject();
      let data: RunOutput;
      let success = false;
      const logPromises = new PromiseSet();
      try {
        const context = new Context(input.ctx);
        if (context.command?.log) {
          globalThis.console = new CaptureLog(
            realConsole,
            context.command?.log,
            logPromises
          );
        } else {
          globalThis.console = realConsole;
        }
        data = { Ok: new Value(await cmd.run(context, jsParams)).M! };
        success = true;
      } catch (error) {
        data = { Err: error.toString() };
        success = false;
      }
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
