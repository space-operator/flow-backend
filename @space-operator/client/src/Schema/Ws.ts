import { Schema } from "effect";
import { bs58, web3 } from "../deps.ts";
import { decodeBase64 } from "@std/encoding/base64";
import { FlowRunId, IValueSchema, NodeId } from "./Common.ts";

// --- WS Response Envelope ---

export const WsResponseOk = <T extends Schema.Schema.Any>(ok: T) =>
  Schema.Struct({
    id: Schema.Number,
    Ok: Schema.optional(ok),
    Err: Schema.optional(Schema.String),
  });

// --- Authentication ---

export const AuthenticateResponseOk = Schema.Struct({
  user_id: Schema.optional(Schema.String),
  pubkey: Schema.optional(Schema.String),
  flow_run_id: Schema.optional(Schema.String),
});
export type AuthenticateResponseOk = typeof AuthenticateResponseOk.Type;

export const AuthenticateResponse = WsResponseOk(AuthenticateResponseOk);
export type AuthenticateResponse = typeof AuthenticateResponse.Type;

// --- Subscribe Flow Run Events ---

export const SubscribeOk = Schema.Struct({
  stream_id: Schema.Number,
});

export const SubscribeFlowRunEventsResponse = WsResponseOk(SubscribeOk);
export type SubscribeFlowRunEventsResponse =
  typeof SubscribeFlowRunEventsResponse.Type;

export const SubscribeSignatureRequestsResponse = WsResponseOk(SubscribeOk);
export type SubscribeSignatureRequestsResponse =
  typeof SubscribeSignatureRequestsResponse.Type;

// --- Log Level ---

export const LogLevel = Schema.Union(
  Schema.Literal("Trace"),
  Schema.Literal("Debug"),
  Schema.Literal("Info"),
  Schema.Literal("Warn"),
  Schema.Literal("Error"),
);
export type LogLevel = typeof LogLevel.Type;

// --- Signature Request ---

export const SignatureRequestSchema = Schema.Struct({
  id: Schema.Number,
  time: Schema.String,
  pubkey: Schema.String,
  message: Schema.String,
  timeout: Schema.Number,
  flow_run_id: Schema.optional(FlowRunId),
  signatures: Schema.optional(
    Schema.Array(
      Schema.Struct({ pubkey: Schema.String, signature: Schema.String }),
    ),
  ),
});
export type ISignatureRequest = typeof SignatureRequestSchema.Type;

/** SignatureRequest with a `buildTransaction()` helper for Solana signing. */
export class SignatureRequest {
  readonly id: number;
  readonly time: string;
  readonly pubkey: string;
  readonly message: string;
  readonly timeout: number;
  readonly flow_run_id?: string;
  readonly signatures?: Array<{ pubkey: string; signature: string }>;

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
    const buffer = decodeBase64(this.message);
    const solMsg = web3.VersionedMessage.deserialize(buffer);

    let sigs: Uint8Array[] | undefined = undefined;
    if (this.signatures) {
      sigs = [];
      const defaultSignature = bs58.encodeBase58(new Uint8Array(64));
      for (let i = 0; i < solMsg.header.numRequiredSignatures; i++) {
        const pubkey = solMsg.staticAccountKeys[i].toBase58();
        let signature = this.signatures.find(
          (x) => x.pubkey === pubkey,
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

// --- Flow Run Events ---

export const FlowStart = Schema.Struct({
  flow_run_id: FlowRunId,
  time: Schema.String,
});
export type FlowStart = typeof FlowStart.Type;

export const FlowError = Schema.Struct({
  flow_run_id: FlowRunId,
  time: Schema.String,
  error: Schema.String,
});
export type FlowError = typeof FlowError.Type;

export const FlowLog = Schema.Struct({
  flow_run_id: FlowRunId,
  time: Schema.String,
  level: LogLevel,
  module: Schema.optional(Schema.String),
  content: Schema.String,
});
export type FlowLog = typeof FlowLog.Type;

export const FlowFinish = Schema.Struct({
  flow_run_id: FlowRunId,
  time: Schema.String,
  not_run: Schema.Array(NodeId),
  output: IValueSchema,
});
export type FlowFinish = typeof FlowFinish.Type;

export const NodeStart = Schema.Struct({
  flow_run_id: FlowRunId,
  time: Schema.String,
  node_id: NodeId,
  times: Schema.Number,
  input: IValueSchema,
});
export type NodeStart = typeof NodeStart.Type;

export const NodeOutput = Schema.Struct({
  flow_run_id: FlowRunId,
  time: Schema.String,
  node_id: NodeId,
  times: Schema.Number,
  output: IValueSchema,
});
export type NodeOutput = typeof NodeOutput.Type;

export const NodeError = Schema.Struct({
  flow_run_id: FlowRunId,
  time: Schema.String,
  node_id: NodeId,
  times: Schema.Number,
  error: Schema.String,
});
export type NodeError = typeof NodeError.Type;

export const NodeLog = Schema.Struct({
  flow_run_id: FlowRunId,
  time: Schema.String,
  node_id: NodeId,
  times: Schema.Number,
  level: LogLevel,
  module: Schema.optional(Schema.String),
  content: Schema.String,
});
export type NodeLog = typeof NodeLog.Type;

export const NodeFinish = Schema.Struct({
  flow_run_id: FlowRunId,
  time: Schema.String,
  node_id: NodeId,
  times: Schema.Number,
});
export type NodeFinish = typeof NodeFinish.Type;

export const ApiInput = Schema.Struct({
  flow_run_id: FlowRunId,
  time: Schema.String,
  url: Schema.String,
});
export type ApiInput = typeof ApiInput.Type;

// --- Discriminated Union for Flow Run Events ---

export const FlowRunEventEnum = Schema.Union(
  Schema.Struct({ event: Schema.Literal("FlowStart"), data: FlowStart }),
  Schema.Struct({ event: Schema.Literal("FlowError"), data: FlowError }),
  Schema.Struct({ event: Schema.Literal("FlowFinish"), data: FlowFinish }),
  Schema.Struct({ event: Schema.Literal("FlowLog"), data: FlowLog }),
  Schema.Struct({ event: Schema.Literal("NodeStart"), data: NodeStart }),
  Schema.Struct({
    event: Schema.Literal("NodeOutput"),
    data: NodeOutput,
  }),
  Schema.Struct({ event: Schema.Literal("NodeError"), data: NodeError }),
  Schema.Struct({
    event: Schema.Literal("NodeFinish"),
    data: NodeFinish,
  }),
  Schema.Struct({ event: Schema.Literal("NodeLog"), data: NodeLog }),
  Schema.Struct({
    event: Schema.Literal("SignatureRequest"),
    data: SignatureRequestSchema,
  }),
  Schema.Struct({ event: Schema.Literal("ApiInput"), data: ApiInput }),
);
export type FlowRunEventEnum = typeof FlowRunEventEnum.Type;

export const FlowRunEvent = Schema.Struct({
  stream_id: Schema.Number,
  event: Schema.String,
  data: Schema.Unknown,
});
export type FlowRunEvent = typeof FlowRunEvent.Type;

// --- Signature Requests Event ---

export const SignatureRequestsEvent = Schema.Struct({
  stream_id: Schema.Number,
  event: Schema.Literal("SignatureRequest"),
  data: SignatureRequestSchema,
});
export type SignatureRequestsEvent = typeof SignatureRequestsEvent.Type;

// --- WS Request Builders ---

export function makeAuthenticateRequest(
  id: number,
  token: string,
): { id: number; method: "Authenticate"; params: { token: string } } {
  return { id, method: "Authenticate", params: { token } };
}

export function makeSubscribeFlowRunEventsRequest(
  id: number,
  flow_run_id: string,
  token?: string,
): {
  id: number;
  method: "SubscribeFlowRunEvents";
  params: { flow_run_id: string; token?: string };
} {
  return {
    id,
    method: "SubscribeFlowRunEvents",
    params: { flow_run_id, token },
  };
}

export function makeSubscribeSignatureRequestsRequest(
  id: number,
): {
  id: number;
  method: "SubscribeSignatureRequests";
  params: Record<string, never>;
} {
  return { id, method: "SubscribeSignatureRequests", params: {} };
}
