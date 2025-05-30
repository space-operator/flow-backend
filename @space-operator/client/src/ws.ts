import type { FlowRunId } from "./types/common.ts";
import {
  AuthenticateRequest,
  type AuthenticateResponse,
  type FlowFinish,
  type FlowRunEvent,
  SignatureRequest,
  type SignatureRequestsEvent,
  SubscribeFlowRunEventsRequest,
  type SubscribeFlowRunEventsResponse,
  SubscribeSignatureRequestsRequest,
  type SubscribeSignatureRequestsResponse,
  type WsResponse,
} from "./types/ws.ts";

export interface WcClientOptions {
  url?: string;
  token?: string | (() => Promise<string>);
  logger?: (msg: string, data: any) => any;
}

export const WS_URL = "wss://dev-api.spaceoperator.com/ws";

function noop() {}

export class WsClient {
  private identity?: AuthenticateResponse["Ok"];
  private logger: Function = noop;
  private url: string;
  private conn?: WebSocket;
  private counter: number = 0;
  private token?: string | (() => Promise<string>);
  private reqCallbacks: Map<number, { resolve: Function; reject: Function }> =
    new Map();
  private streamCallbacks: Map<number, { callback: Function }> = new Map();
  private queue: Array<string> = [];
  private futureStreams: Map<number, any[]> = new Map();
  private closed: Promise<void>;
  private resolveClosed?: Function;

  constructor(options: WcClientOptions) {
    this.url = options.url ?? WS_URL;
    this.token = options.token;
    this.logger = options.logger ?? noop;
    this.closed = new Promise((resolve, _) => {
      this.resolveClosed = resolve;
    });
  }

  public async close() {
    if (this.conn) {
      this.conn.close();
      await this.closed;
    }
  }

  public getIdentity(): WsClient["identity"] {
    return this.identity;
  }

  public setLogger(logger: Function) {
    this.logger = logger;
  }

  public setToken(token: string | (() => Promise<string>)) {
    this.token = token;
  }

  private newStream(id: number, c: { callback: Function }) {
    this.streamCallbacks.set(id, c);
    const msgs = this.futureStreams.get(id);
    if (msgs != null) {
      msgs.forEach((value) => c.callback(value));
      this.futureStreams.delete(id);
    }
  }

  public async subscribeFlowRunEvents(
    callback: (ev: FlowRunEvent) => any,
    id: FlowRunId,
    token?: string,
    finishCallback?: (ev: FlowFinish) => any,
  ) {
    const result: SubscribeFlowRunEventsResponse = await this.send(
      new SubscribeFlowRunEventsRequest(this.nextId(), id, token),
    );
    if (result.Err !== undefined) {
      throw new Error(result.Err);
    }
    if (result.Ok !== undefined) {
      const id = result.Ok.stream_id;
      this.newStream(id, {
        callback: (ev: FlowRunEvent) => {
          if (ev.event === "SignatureRequest") {
            ev.data = new SignatureRequest(ev.data);
          }
          callback(ev);
          if (ev.event === "FlowFinish") {
            if (finishCallback !== undefined) {
              finishCallback(ev.data);
              this.streamCallbacks.delete(id);
            }
          }
        },
      });
    } else {
      throw new Error("response must contains Ok or Err");
    }
  }

  public async subscribeSignatureRequest(
    callback: (ev: SignatureRequestsEvent) => any,
  ) {
    const result: SubscribeSignatureRequestsResponse = await this.send(
      new SubscribeSignatureRequestsRequest(this.nextId()),
    );
    if (result.Err !== undefined) {
      throw new Error(result.Err);
    }
    if (result.Ok !== undefined) {
      this.newStream(result.Ok.stream_id, {
        callback: (ev: SignatureRequestsEvent) => {
          if (ev.event === "SignatureRequest") {
            ev.data = new SignatureRequest(ev.data);
          }
          callback(ev);
        },
      });
    } else {
      throw new Error("response must contains Ok or Err");
    }
  }

  private async getToken(): Promise<string> {
    switch (typeof this.token) {
      case "undefined":
        throw new Error("no authentication token");
      case "string":
        return this.token;
      case "function":
        return await this.token();
      default:
        throw new TypeError("invalid token type");
    }
  }

  private connect() {
    if (this.conn !== undefined) return;
    this.conn = new WebSocket(this.url);
    this.conn.onopen = () => this.onConnOpen();
    this.conn.onmessage = (event) => this.onConnMessage(event);
    this.conn.onerror = (error) => this.onConnError(error);
    this.conn.onclose = (event) => this.onConnClose(event);
  }

  private disconnect() {
    this.conn = undefined;
    this.reqCallbacks.forEach(({ reject }) => {
      reject("disconnected");
    });
    this.streamCallbacks.clear();
    this.reqCallbacks.clear();
    if (this.resolveClosed) this.resolveClosed();
  }

  private onConnClose(event: any) {
    this.log("closing", event);
    this.disconnect();
  }

  private onConnError(event: any) {
    this.log("error", event);
    this.disconnect();
  }

  private onConnMessage(msg: { data: any }) {
    if (typeof msg.data === "string") {
      const json = JSON.parse(msg.data);
      this.log("received", json);
      if (json.id != null) {
        const cb = this.reqCallbacks.get(json.id);
        if (cb != null) {
          this.reqCallbacks.delete(json.id);
          cb.resolve(json);
        } else {
          throw `no callback for req ${json.id}`;
        }
      } else if (json.stream_id != null) {
        const cb = this.streamCallbacks.get(json.stream_id);
        if (cb != null) {
          cb.callback(json);
        } else {
          const msgs = this.futureStreams.get(json.stream_id);
          if (msgs != null) {
            msgs.push(json);
          } else {
            this.futureStreams.set(json.stream_id, [json]);
          }
        }
      } else {
        throw new Error("invalid message");
      }
    }
  }

  private log(msg: string, data?: any) {
    this.logger(msg, data);
  }

  private nextId(): number {
    this.counter += 1;
    if (this.counter > 0xffffffff) this.counter = 0;
    return this.counter;
  }

  private async send(msg: {
    id: number;
    method: string;
    params: any;
  }): Promise<WsResponse<any>> {
    const text = JSON.stringify(msg);
    if (this.conn !== undefined) {
      this.log("sending", msg);
      this.conn.send(text);
    } else {
      this.log("queueing", msg);
      this.queue.push(text);
      this.connect();
    }
    return await new Promise((resolve, reject) => {
      this.reqCallbacks.set(msg.id, { resolve, reject });
    });
  }

  async authenticate() {
    if (this.token === undefined) return;
    const token = await this.getToken();
    const result: AuthenticateResponse = await this.send(
      new AuthenticateRequest(this.nextId(), token),
    );
    if (result.Err !== undefined) {
      console.error("Authenticate error", result.Err);
    }
    if (result.Ok !== undefined) {
      this.identity = result.Ok;
    } else {
      throw new Error("response must contains Ok or Err");
    }
  }

  private async onConnOpen() {
    await this.authenticate();
    this.queue = this.queue.filter((msg) => {
      if (this.conn !== undefined) {
        this.log("sending queued message", msg);
        this.conn.send(msg);
        return false;
      } else {
        return true;
      }
    });
  }
}
