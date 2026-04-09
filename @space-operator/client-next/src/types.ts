import {
  decodeBase58,
  decodeBase64,
  type IValue,
  Value,
  web3,
} from "./deps.ts";
import type { SupabaseSession } from "./deps.ts";

export type FlowId = string;
export type DeploymentId = string;
export type FlowRunId = string;
export type NodeId = string;
export type UserId = string;

export type MaybePromise<T> = T | Promise<T>;
export type ValueProvider<T> = T | (() => MaybePromise<T>);
export type StringProvider = ValueProvider<string>;
export type PublicKeyInput = string | web3.PublicKey;
export type PublicKeyProvider = ValueProvider<PublicKeyInput>;

export interface ApiKeyAuth {
  kind: "apiKey";
  apiKey: StringProvider;
}

export interface BearerAuth {
  kind: "bearer";
  token: StringProvider;
}

export interface FlowRunTokenAuth {
  kind: "flowRunToken";
  token: StringProvider;
}

export interface PublicKeyAuth {
  kind: "publicKey";
  publicKey: PublicKeyProvider;
}

export type AuthStrategy =
  | ApiKeyAuth
  | BearerAuth
  | FlowRunTokenAuth
  | PublicKeyAuth;

export interface RetryPolicy {
  attempts?: number;
  backoffMs?: number | ((attempt: number) => number);
  retryableStatusCodes?: number[];
}

export interface ClientLoggerEvent {
  scope: "http" | "ws";
  event: string;
  data?: unknown;
}

export type ClientLogger = (entry: ClientLoggerEvent) => void;

export interface WebSocketMessageEventLike {
  data: unknown;
}

export interface WebSocketCloseEventLike {
  code?: number;
  reason?: string;
}

export interface WebSocketLike {
  onopen: ((event?: unknown) => void) | null;
  onmessage: ((event: WebSocketMessageEventLike) => void) | null;
  onerror: ((event: unknown) => void) | null;
  onclose: ((event: WebSocketCloseEventLike) => void) | null;
  send(data: string): void;
  close(code?: number, reason?: string): void;
}

export type WebSocketFactory = (url: string) => WebSocketLike;

export interface WebSocketIdentity {
  user_id?: UserId;
  pubkey?: string;
  flow_run_id?: FlowRunId;
}

export interface CreateClientOptions {
  baseUrl: string;
  auth?: AuthStrategy;
  anonKey?: StringProvider;
  fetch?: typeof globalThis.fetch;
  webSocketFactory?: WebSocketFactory;
  logger?: ClientLogger;
  telemetry?: ClientTelemetryOptions;
  retry?: RetryPolicy;
  timeoutMs?: number;
}

export interface ClientTelemetryOptions {
  tracer?: import("@opentelemetry/api").Tracer;
  tracerName?: string;
  tracerVersion?: string;
  attributes?: Record<string, string | number | boolean>;
}

export interface RequestOptions {
  auth?: AuthStrategy;
  headers?: HeadersInit;
  signal?: AbortSignal;
  retry?: RetryPolicy;
  timeoutMs?: number;
}

export interface SubscribeFlowRunOptions extends RequestOptions {
  token?: StringProvider;
}

export interface JsonObject {
  [key: string]: JsonValue;
}

export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonObject
  | JsonValue[];

export type FlowValueInput = Value | IValue | unknown;
export type FlowInputs = Record<string, FlowValueInput>;

export interface ValuesConfig {
  nodes: Record<NodeId, FlowRunId>;
  default_run_id?: FlowRunId;
}

export interface PartialConfig {
  only_nodes: NodeId[];
  values_config: ValuesConfig;
}

export interface StartFlowParams {
  inputs?: FlowInputs;
  partial_config?: PartialConfig;
  environment?: Record<string, string>;
  output_instructions?: boolean;
}

export interface StartFlowSharedParams {
  inputs?: FlowInputs;
  output_instructions?: boolean;
}

export interface ReadFlowParams {
  inputs?: FlowInputs;
  skipCache?: boolean;
}

export interface SolanaActionConfig {
  action_signer: string;
  action_identity: string;
}

export interface StartFlowUnverifiedParams {
  inputs?: FlowInputs;
  output_instructions?: boolean;
  action_identity?: string;
  action_config?: SolanaActionConfig;
  fees?: Array<[string, number]>;
}

export interface StopFlowParams {
  timeout_millies?: number;
  reason?: string;
}

export type DeploymentSpecifier =
  | { id: DeploymentId }
  | { flow: FlowId; tag?: string };

export interface StartDeploymentParams {
  inputs?: FlowInputs;
  action_signer?: string;
}

export interface ReadDeploymentParams {
  inputs?: FlowInputs;
  skipCache?: boolean;
}

export interface ClaimTokenOutput {
  user_id: UserId;
  access_token: string;
  refresh_token: string;
  expires_at: number;
}

export interface ConfirmAuthOutput {
  session: SupabaseSession;
  new_user: boolean;
}

export interface SuccessResponse {
  success: true;
}

export interface FlowRunStartOutput {
  flow_run_id: FlowRunId;
}

export interface FlowRunTokenOutput extends FlowRunStartOutput {
  token: string;
}

export interface CloneFlowOutput {
  flow_id: FlowId;
  id_map: Record<FlowId, FlowId>;
}

export interface ApiKeyRecord {
  key_hash: string;
  trimmed_key: string;
  name: string;
  user_id: UserId;
  created_at: string;
}

export interface CreateApiKeyOutput extends ApiKeyRecord {
  full_key: string;
}

export interface ApiKeyInfoOutput {
  user_id: UserId;
}

export interface IrohInfo {
  node_id: string;
  relay_url: string;
  direct_addresses: string[];
}

export interface ServiceInfoOutput {
  supabase_url: string;
  anon_key: string;
  iroh: IrohInfo;
  base_url: string;
}

export interface SubmitSignatureInput {
  id: number;
  signature: string | Uint8Array | ArrayBuffer;
  new_msg?: string | Uint8Array | ArrayBuffer;
}

export type SignatureRequestKind = "transaction_message" | "message";

export interface WalletUpsertBody {
  [key: string]: unknown;
}

export interface WsResponse<T> {
  id: number;
  Ok?: T;
  Err?: string;
}

export type LogLevel = "Trace" | "Debug" | "Info" | "Warn" | "Error";

export interface ISignatureRequest {
  id: number;
  time: string;
  pubkey: string;
  message: string;
  timeout: number;
  kind?: SignatureRequestKind;
  flow_run_id?: FlowRunId;
  signatures?: Array<{ pubkey: string; signature: string }>;
}

export class SignatureRequest implements ISignatureRequest {
  id: number;
  time: string;
  pubkey: string;
  message: string;
  timeout: number;
  kind: SignatureRequestKind;
  flow_run_id?: FlowRunId;
  signatures?: Array<{ pubkey: string; signature: string }>;

  constructor(value: ISignatureRequest) {
    this.id = value.id;
    this.time = value.time;
    this.pubkey = value.pubkey;
    this.message = value.message;
    this.timeout = value.timeout;
    this.kind = value.kind ?? "transaction_message";
    this.flow_run_id = value.flow_run_id;
    this.signatures = value.signatures;
  }

  buildTransaction(): web3.VersionedTransaction {
    if (this.kind !== "transaction_message") {
      throw new Error(
        `signature request ${this.id} is ${this.kind}, not transaction_message`,
      );
    }

    const buffer = decodeBase64(this.message);
    const solMsg = web3.VersionedMessage.deserialize(buffer);

    let sigs: Uint8Array[] | undefined;
    if (this.signatures) {
      sigs = [];
      const defaultSignature = new Uint8Array(64);
      for (let i = 0; i < solMsg.header.numRequiredSignatures; i += 1) {
        const pubkey = solMsg.staticAccountKeys[i].toBase58();
        const signature = this.signatures.find((item) => item.pubkey === pubkey)
          ?.signature;
        if (signature === undefined) {
          sigs.push(defaultSignature);
        } else {
          sigs.push(decodeBase58(signature));
        }
      }
    }

    return new web3.VersionedTransaction(solMsg, sigs);
  }

  buildMessage(): Uint8Array {
    if (this.kind !== "message") {
      throw new Error(
        `signature request ${this.id} is ${this.kind}, not message`,
      );
    }

    return decodeBase64(this.message);
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
  not_run: NodeId[];
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

export interface ApiInputEvent {
  flow_run_id: FlowRunId;
  time: string;
  url: string;
}

export interface ReadResult {
  value: Value;
  cached: boolean;
  etag?: string;
  cacheControl?: string;
  lastModified?: string;
}

export type FlowRunEvent =
  | { stream_id: number; event: "FlowStart"; data: FlowStart }
  | { stream_id: number; event: "FlowError"; data: FlowError }
  | { stream_id: number; event: "FlowFinish"; data: FlowFinish }
  | { stream_id: number; event: "FlowLog"; data: FlowLog }
  | { stream_id: number; event: "NodeStart"; data: NodeStart }
  | { stream_id: number; event: "NodeOutput"; data: NodeOutput }
  | { stream_id: number; event: "NodeError"; data: NodeError }
  | { stream_id: number; event: "NodeFinish"; data: NodeFinish }
  | { stream_id: number; event: "NodeLog"; data: NodeLog }
  | { stream_id: number; event: "SignatureRequest"; data: SignatureRequest }
  | { stream_id: number; event: "ApiInput"; data: ApiInputEvent };

export interface SignatureRequestsEvent {
  stream_id: number;
  event: "SignatureRequest";
  data: SignatureRequest;
}
