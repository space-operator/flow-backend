import type { ServiceProxy } from "./deps.ts";

type Console = typeof globalThis.console;
type Level = "INFO" | "ERROR" | "WARN" | "DEBUG" | "TRACE";

export class PromiseSet {
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

export class CaptureLog implements Console {
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
