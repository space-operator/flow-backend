import { bs58, lib, web3, Value, IValue } from "./deps.ts";
import { FlowId, FlowRunId, RestResult } from "./types/common.ts";
import { StartFlowOutput, StartFlowParams } from "./types/rest/start-flow.ts";
import {
  StartFlowUnverifiedOutput,
  StartFlowUnverifiedParams,
} from "./types/rest/start-flow-unverified.ts";
import {
  StartFlowSharedOutput,
  StartFlowSharedParams,
} from "./types/rest/start-flow-shared.ts";
import { StopFlowOutput, StopFlowParams } from "./types/rest/stop-flow.ts";
import {
  SubmitSignatureOutput,
  SubmitSignatureParams,
} from "./types/rest/submit-signature.ts";
import { SignatureRequest } from "./types/ws.ts";
import { GetFlowOutputOutput } from "./types/rest/get-flow-output.ts";
import { ErrorBody, ISignatureRequest } from "./mod.ts";

export interface ClientOptions {
  host?: string;
  token?: string | (() => Promise<string>);
}

const HOST = "https://dev-api.spaceoperator.com";

function noop() {}

export type TokenProvider = string | (() => Promise<string>);

async function parseResponse<T>(resp: Response): Promise<T> {
  if (resp.status !== 200) {
    let error;
    if (resp.headers.get("content-type") === "application/json") {
      error = ((await resp.json()) as ErrorBody).error;
    } else {
      error = await resp.text();
    }
    if (error === undefined) {
      error = "An error occurred";
    }
    throw new Error(error);
  }
  return await resp.json();
}

export class Client {
  host: string;
  token?: TokenProvider;
  private logger: Function = noop;

  constructor(options: ClientOptions) {
    this.host = options.host ?? HOST;
    this.token = options.token;
  }

  setToken(token: string | (() => Promise<string>)) {
    this.token = token;
  }

  async getToken(): Promise<string> {
    switch (typeof this.token) {
      case "undefined":
        throw new Error("no authentication token");
      case "string":
        return this.token;
      case "function":
        return await this.token();
      default:
        throw new Error("invalid token type");
    }
  }

  public setLogger(logger: Function) {
    this.logger = logger;
  }

  async #sendJSONGet<T>(
    url: string,
    auth: boolean | TokenProvider = true
  ): Promise<T> {
    const req = new Request(url);
    switch (typeof auth) {
      case "boolean":
        if (auth === true) {
          req.headers.set("authorization", await this.getToken());
        }
        break;
      case "string":
        req.headers.set("authorization", auth);
        break;
      case "function":
        req.headers.set("authorization", await auth());
        break;
      default:
        throw new TypeError("unexpected type");
    }

    const resp = await fetch(req);
    return await parseResponse(resp);
  }

  async #sendJSONPost<T>(
    url: string,
    body: any,
    auth: boolean | TokenProvider = true
  ): Promise<T> {
    const req = new Request(url, {
      method: "POST",
      body: JSON.stringify(body),
      headers: {
        "content-type": "application/json",
      },
    });
    switch (typeof auth) {
      case "boolean":
        if (auth === true) {
          req.headers.set("authorization", await this.getToken());
        }
        break;
      case "string":
        req.headers.set("authorization", auth);
        break;
      case "function":
        req.headers.set("authorization", await auth());
        break;
      default:
        throw new TypeError("unexpected type");
    }

    const resp = await fetch(req);
    return await parseResponse(resp);
  }

  async startFlow(
    id: FlowId,
    params: StartFlowParams
  ): Promise<StartFlowOutput> {
    return await this.#sendJSONPost(`${this.host}/flow/start/${id}`, params);
  }

  async startFlowShared(
    id: FlowId,
    params: StartFlowSharedParams
  ): Promise<StartFlowSharedOutput> {
    return await this.#sendJSONPost(
      `${this.host}/flow/start_shared/${id}`,
      params
    );
  }

  async startFlowUnverified(
    id: FlowId,
    publicKey: web3.PublicKey,
    params: StartFlowUnverifiedParams
  ): Promise<StartFlowUnverifiedOutput> {
    return await this.#sendJSONPost(
      `${this.host}/flow/start_unverified/${id}`,
      params,
      publicKey.toBase58()
    );
  }

  async getFlowOutput(
    runId: FlowRunId,
    token?: string
  ): Promise<GetFlowOutputOutput> {
    const value: IValue = await this.#sendJSONGet(
      `${this.host}/flow/output/${runId}`,
      token ?? (await this.getToken())
    );
    return Value.fromJSON(value);
  }

  async getSignatureRequest(
    runId: FlowRunId,
    token?: string
  ): Promise<SignatureRequest> {
    const value: ISignatureRequest = await this.#sendJSONGet(
      `${this.host}/flow/signature_request/${runId}`,
      token ?? (await this.getToken())
    );
    return new SignatureRequest(value);
  }

  async stopFlow(
    runId: FlowRunId,
    params: StopFlowParams
  ): Promise<StopFlowOutput> {
    return await this.#sendJSONPost(`${this.host}/flow/stop/${runId}`, params);
  }

  async submitSignature(
    params: SubmitSignatureParams
  ): Promise<SubmitSignatureOutput> {
    return await this.#sendJSONPost(`${this.host}/signature/submit`, params);
  }

  async signAndSubmitSignature(
    req: SignatureRequest,
    publicKey: web3.PublicKey,
    signTransaction: (tx: web3.Transaction) => Promise<web3.Transaction>
  ) {
    const requestedPublicKey = new web3.PublicKey(req.pubkey);
    if (!publicKey.equals(requestedPublicKey)) {
      throw new Error(
        `different public key:\nrequested: ${
          req.pubkey
        }}\nwallet: ${publicKey.toBase58()}`
      );
    }
    const tx = req.buildTransaction();
    this.logger("tx", tx);
    const signedTx: web3.Transaction = await signTransaction(tx);
    this.logger("signedTx", signedTx);
    const signature = signedTx.signatures.find(({ publicKey }) =>
      publicKey.equals(requestedPublicKey)
    )?.signature;
    if (signature == null) throw new Error("signature is null");

    const before = tx.serializeMessage();
    const after = signedTx.serializeMessage();
    const new_msg = before.equals(after) ? undefined : after.toString("base64");
    await this.submitSignature({
      id: req.id,
      signature: bs58.encodeBase58(signature),
      new_msg,
    });
  }
}
