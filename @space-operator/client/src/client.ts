import { bs58, web3, Value, type IValue, Buffer } from "./deps.ts";
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
  type DeploymentId,
  DeploymentSpecifier,
  SignatureRequest,
  type InitAuthOutput,
  type ConfirmAuthOutput,
  type StartDeploymentParams,
  type StartDeploymentOutput,
  type IDeploymentSpecifier,
  Database,
} from "./mod.ts";

export type TokenProvider = string | (() => Promise<string>);

function header(key: string): string {
  if (key.startsWith("b3-")) {
    return "x-api-key";
  } else {
    return "authorization";
  }
}

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

export interface ClaimTokenOutput {
  access_token: string;
  refresh_token: string;
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

  async #setAuthHeader(
    req: Request,
    auth: boolean | TokenProvider = true
  ): Promise<Request> {
    switch (typeof auth) {
      case "boolean":
        if (auth === true) {
          const token = await getToken(this.token);
          req.headers.set(header(token), token);
        }
        break;
      case "string":
        req.headers.set(header(auth), auth);
        break;
      case "function": {
        const token = await auth();
        req.headers.set(header(token), token);
        break;
      }
      default:
        throw new TypeError("unexpected type");
    }
    return req;
  }

  async #sendJSONGet<T>(
    url: string,
    auth: boolean | TokenProvider = true
  ): Promise<T> {
    let req = new Request(url);
    req = await this.#setAuthHeader(req, auth);

    const resp = await fetch(req);
    return await parseResponse(resp);
  }

  async #sendJSONPost<T>(
    url: string,
    body?: any,
    auth: boolean | TokenProvider = true,
    anonKey: boolean = false
  ): Promise<T> {
    const reqBody = body !== undefined ? JSON.stringify(body) : undefined;
    const headers: Record<string, string> =
      body !== undefined
        ? {
            "content-type": "application/json",
          }
        : {};
    let req = new Request(url, {
      method: "POST",
      body: reqBody,
      headers,
    });
    req = await this.#setAuthHeader(req, auth);

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
    signTransaction: (
      tx: web3.VersionedTransaction
    ) => Promise<web3.VersionedTransaction>
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
    const signerPosition = tx.message.staticAccountKeys.findIndex((pk) =>
      pk.equals(requestedPublicKey)
    );
    if (signerPosition === -1) {
      throw new Error("pubkey is not in signers list");
    }
    this.logger("tx", tx);
    const signedTx: web3.VersionedTransaction = await signTransaction(tx);
    this.logger("signedTx", signedTx);
    const signature = signedTx.signatures[signerPosition];
    if (signature == null) throw new Error("signature is null");

    const before = Buffer.from(tx.message.serialize());
    const after = Buffer.from(signedTx.message.serialize());
    const new_msg = before.equals(after) ? undefined : after.toString("base64");
    await this.submitSignature({
      id: req.id,
      signature: bs58.encodeBase58(signature),
      new_msg,
    });
  }

  async deployFlow(id: FlowId): Promise<DeploymentId> {
    const result: {
      deployment_id: string;
    } = await this.#sendJSONPost(`${this.host}/flow/deploy/${id}`, {});
    return result.deployment_id;
  }

  async startDeployment(
    deployment: IDeploymentSpecifier,
    params?: StartDeploymentParams
  ): Promise<StartDeploymentOutput> {
    return await this.#sendJSONPost(
      `${this.host}/deployment/start?${new DeploymentSpecifier(
        deployment
      ).formatQuery()}`,
      params
    );
  }

  async claimToken(): Promise<ClaimTokenOutput> {
    return await this.#sendJSONPost(`${this.host}/auth/claim_token`);
  }
}
