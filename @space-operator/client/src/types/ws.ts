import type { FlowRunId, NodeId } from "./common.ts";
import { type Value, Buffer, bs58, web3 } from "../deps.ts";

export interface WsResponse<T> {
  id: number;
  Ok?: T;
  Err?: string;
}

export class AuthenticateRequest {
  id: number;
  method: "Authenticate";
  params: {
    token: string;
  };
  constructor(id: number, token: string) {
    this.id = id;
    this.method = "Authenticate";
    this.params = { token };
  }
}

export interface AuthenticateResponse {
  id: number;
  Ok?: {
    user_id?: string;
    pubkey?: string;
    flow_run_id: string;
  };
  Err?: string;
}

export class SubscribeFlowRunEventsRequest {
  id: number;
  method: "SubscribeFlowRunEvents";
  params: {
    flow_run_id: string;
    token?: string;
  };
  constructor(id: number, flow_run_id: string, token?: string) {
    this.id = id;
    this.method = "SubscribeFlowRunEvents";
    this.params = {
      flow_run_id,
      token,
    };
  }
}

export interface SubscribeFlowRunEventsResponse {
  id: number;
  Ok?: {
    stream_id: number;
  };
  Err?: string;
}

export class SubscribeSignatureRequestsRequest {
  id: number;
  method: "Authenticate";
  params: {};
  constructor(id: number) {
    this.id = id;
    this.method = "Authenticate";
    this.params = {};
  }
}

export interface SubscribeSignatureRequestsResponse {
  id: number;
  Ok?: {
    stream_id: number;
  };
  Err?: string;
}

export interface SignatureRequestsEvent {
  stream_id: number;
  event: "SignatureRequest";
  data: SignatureRequest;
}

export type FlowRunEvent = { stream_id: number } & FlowRunEventEnum;

export type FlowRunEventEnum =
  | {
      event: "FlowStart";
      data: FlowStart;
    }
  | {
      event: "FlowError";
      data: FlowLog;
    }
  | {
      event: "FlowFinish";
      data: FlowFinish;
    }
  | {
      event: "FlowLog";
      data: FlowLog;
    }
  | {
      event: "NodeStart";
      data: NodeStart;
    }
  | {
      event: "NodeOutput";
      data: NodeOutput;
    }
  | {
      event: "NodeError";
      data: NodeError;
    }
  | {
      event: "NodeFinish";
      data: NodeFinish;
    }
  | {
      event: "NodeLog";
      data: NodeLog;
    }
  | {
      event: "SignatureRequest";
      data: SignatureRequest;
    };

export type LogLevel = "Trace" | "Debug" | "Info" | "Warn" | "Error";

export interface ISignatureRequest {
  id: number;
  time: string;
  pubkey: string;
  message: string;
  timeout: number;
  flow_run_id?: FlowRunId;
  signatures?: Array<{ pubkey: string; signature: string }>;
}

export class SignatureRequest implements ISignatureRequest {
  id: number;
  time: string;
  pubkey: string;
  message: string;
  timeout: number;
  flow_run_id?: FlowRunId;
  signatures?: Array<{ pubkey: string; signature: string }>;
  constructor(x: ISignatureRequest) {
    this.id = x.id;
    this.time = x.time;
    this.pubkey = x.pubkey;
    this.message = x.message;
    this.timeout = x.timeout;
    this.flow_run_id = x.flow_run_id;
    this.signatures = x.signatures;
  }

  buildTransaction(): web3.VersionedTransaction {
    const buffer = Buffer.from(this.message, "base64");
    const solMsg = web3.VersionedMessage.deserialize(buffer);

    let sigs = undefined;
    if (this.signatures) {
      sigs = [];
      const defaultSignature = bs58.encodeBase58(new Uint8Array(64));
      for (let i = 0; i < solMsg.header.numRequiredSignatures; i++) {
        const pubkey = solMsg.staticAccountKeys[i].toBase58();
        let signature = this.signatures.find(
          (x) => x.pubkey === pubkey
        )?.signature;
        if (signature === undefined) {
          signature = defaultSignature;
        }
        sigs.push(bs58.decodeBase58(signature));
      }
    }

    return new web3.VersionedTransaction(solMsg, sigs);
  }
}

export interface FlowStart {
  flow_run_id: FlowRunId;
  time: string;
}

export interface FlowError {
  flow_run_id: FlowRunId;
  time: string;
  error: string;
}

export interface FlowLog {
  flow_run_id: FlowRunId;
  time: string;
  level: LogLevel;
  module?: string;
  content: string;
}

export interface FlowFinish {
  flow_run_id: FlowRunId;
  time: string;
  not_run: Array<NodeId>;
  output: Value;
}

export interface NodeStart {
  flow_run_id: FlowRunId;
  time: string;
  node_id: NodeId;
  times: number;
  input: Value;
}

export interface NodeOutput {
  flow_run_id: FlowRunId;
  time: string;
  node_id: NodeId;
  times: number;
  output: Value;
}

export interface NodeError {
  flow_run_id: FlowRunId;
  time: string;
  node_id: NodeId;
  times: number;
  error: string;
}

export interface NodeLog {
  flow_run_id: FlowRunId;
  time: string;
  node_id: NodeId;
  times: number;
  level: LogLevel;
  module?: string;
  content: string;
}

export interface NodeFinish {
  flow_run_id: FlowRunId;
  time: string;
  node_id: NodeId;
  times: number;
}
