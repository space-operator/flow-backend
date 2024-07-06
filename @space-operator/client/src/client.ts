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

export interface ClientOptions {
  host?: string;
  token?: string | (() => Promise<string>);
}

const HOST = "https://dev-api.spaceoperator.com";

function noop() {}

export class Client {
  host: string;
  token?: string | (() => Promise<string>);
  private logger: Function = noop;

  constructor(options: ClientOptions) {
    this.host = options.host ?? HOST;
    this.token = options.token;
  }

  setToken(token: string | (() => Promise<string>)) {
    this.token = token;
  }

  async getToken(): Promise<string | null> {
    if (this.token == null) return null;
    switch (typeof this.token) {
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

  async startFlow(
    id: FlowId,
    params: StartFlowParams
  ): Promise<RestResult<StartFlowOutput>> {
    try {
      const token = await this.getToken();
      if (token == null) {
        throw new Error("no authentication token");
      }
      const resp = await fetch(`${this.host}/flow/start/${id}`, {
        method: "POST",
        headers: {
          authorization: token,
          "content-type": "application/json",
        },
        body: JSON.stringify(params),
      });
      return await resp.json();
    } catch (error: any) {
      return { error: error.toString() };
    }
  }

  async startFlowShared(
    id: FlowId,
    params: StartFlowSharedParams
  ): Promise<RestResult<StartFlowSharedOutput>> {
    try {
      const token = await this.getToken();
      if (token == null) {
        throw new Error("no authentication token");
      }
      const resp = await fetch(`${this.host}/flow/start_shared/${id}`, {
        method: "POST",
        headers: {
          authorization: token,
          "content-type": "application/json",
        },
        body: JSON.stringify(params),
      });
      return await resp.json();
    } catch (error: any) {
      return { error: error.toString() };
    }
  }

  async startFlowUnverified(
    id: FlowId,
    publicKey: web3.PublicKey,
    params: StartFlowUnverifiedParams
  ): Promise<RestResult<StartFlowUnverifiedOutput>> {
    try {
      const resp = await fetch(`${this.host}/flow/start_unverified/${id}`, {
        method: "POST",
        headers: {
          authorization: publicKey.toBase58(),
          "content-type": "application/json",
        },
        body: JSON.stringify(params),
      });
      return await resp.json();
    } catch (error: any) {
      return { error: error.toString() };
    }
  }

  async getFlowOutput(
    runId: FlowRunId,
    token?: string
  ): Promise<RestResult<GetFlowOutputOutput>> {
    try {
      if (token == null) {
        token = (await this.getToken()) as any;
      }
      if (token == null) {
        throw new Error("no authentication token");
      }
      const resp = await fetch(`${this.host}/flow/output/${runId}`, {
        method: "GET",
        headers: {
          authorization: token,
        },
      });
      const value: IValue = await resp.json();
      return Value.fromJSON(value);
    } catch (error: any) {
      return { error: error.toString() };
    }
  }

  async stopFlow(
    runId: FlowRunId,
    params: StopFlowParams
  ): Promise<RestResult<StopFlowOutput>> {
    try {
      const token = await this.getToken();
      if (token == null) {
        throw new Error("no authentication token");
      }
      const resp = await fetch(`${this.host}/flow/stop/${runId}`, {
        method: "POST",
        headers: {
          authorization: token,
          "content-type": "application/json",
        },
        body: JSON.stringify(params),
      });
      return await resp.json();
    } catch (error: any) {
      return { error: error.toString() };
    }
  }

  async submitSignature(
    params: SubmitSignatureParams
  ): Promise<RestResult<SubmitSignatureOutput>> {
    try {
      const resp = await fetch(`${this.host}/signature/submit`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
        },
        body: JSON.stringify(params),
      });
      return await resp.json();
    } catch (error: any) {
      return { error: error.toString() };
    }
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
