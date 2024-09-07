import { bs58, web3, Value, type IValue } from "./deps.ts";
import {
  type ErrorBody,
  type ISignatureRequest,
  type StartFlowOutput,
  type StartFlowParams,
  type StartFlowSharedOutput,
  type StartFlowSharedParams,
  type StartFlowUnverifiedOutput,
  type StartFlowUnverifiedParams,
  type StopFlowOutput,
  type StopFlowParams,
  type SubmitSignatureOutput,
  type SubmitSignatureParams,
  type GetFlowOutputOutput,
  type FlowId,
  type FlowRunId,
  SignatureRequest,
  type InitAuthOutput,
  type ConfirmAuthOutput,
} from "./mod.ts";

export type TokenProvider = string | (() => Promise<string>);

async function getToken(token?: TokenProvider): Promise<string> {
  switch (typeof token) {
    case "undefined":
      throw new Error("no authentication token");
    case "string":
      return token;
    case "function":
      return await token();
    default:
      throw new Error("invalid token type");
  }
}

export interface ClientOptions {
  host?: string;
  // Authorization token
  token?: TokenProvider;
  // Supabase Anon key
  anonKey?: TokenProvider;
}

const HOST = "https://dev-api.spaceoperator.com";

function noop() {}

async function parseResponse<T>(resp: Response): Promise<T> {
  if (resp.status !== 200 && resp.status !== 201) {
    let error;
    if (resp.headers.get("content-type") === "application/json") {
      const body = await resp.json();
      error = (body as ErrorBody).error ?? JSON.stringify(body);
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
  anonKey?: TokenProvider;
  private logger: Function = noop;

  constructor(options: ClientOptions = {}) {
    this.host = options.host ?? HOST;
    this.token = options.token;
    this.anonKey = options.anonKey;
  }

  async upsertWallet(body: any): Promise<any> {
    return await this.#sendJSONPost(`${this.host}/wallets/upsert`, body);
  }

  setToken(token: string | (() => Promise<string>)) {
    this.token = token;
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
          req.headers.set("authorization", await getToken(this.token));
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
    auth: boolean | TokenProvider = true,
    anonKey: boolean = false
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
          req.headers.set("authorization", await getToken(this.token));
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
    if (anonKey === true) {
      req.headers.set("apikey", await getToken(this.anonKey));
    }

    const resp = await fetch(req);
    return await parseResponse(resp);
  }

  async initAuth(pubkey: web3.PublicKey | string): Promise<string> {
    let pubkeyBs58;
    if (typeof pubkey === "string") {
      pubkeyBs58 = pubkey;
    } else {
      pubkeyBs58 = pubkey.toBase58();
    }
    return (
      (await this.#sendJSONPost(
        `${this.host}/auth/init`,
        {
          pubkey: pubkeyBs58,
        },
        false,
        true
      )) as InitAuthOutput
    ).msg;
  }

  async confirmAuth(
    msg: string,
    signature: ArrayBuffer | Uint8Array | string
  ): Promise<ConfirmAuthOutput> {
    let sig;
    if (typeof signature === "string") {
      sig = signature;
    } else {
      sig = bs58.encodeBase58(signature);
    }
    const token = `${msg}.${sig}`;
    return await this.#sendJSONPost(
      `${this.host}/auth/confirm`,
      {
        token,
      },
      false,
      true
    );
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
      token ?? true
    );
    return Value.fromJSON(value);
  }

  async getSignatureRequest(
    runId: FlowRunId,
    token?: string
  ): Promise<SignatureRequest> {
    const value: ISignatureRequest = await this.#sendJSONGet(
      `${this.host}/flow/signature_request/${runId}`,
      token ?? true
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
    return await this.#sendJSONPost(
      `${this.host}/signature/submit`,
      params,
      false
    );
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
