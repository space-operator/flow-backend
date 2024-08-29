import { Buffer, naclSign, web3 } from "./deps.ts";

export { Value, type IValue } from "./deps.ts";
export { Client, type ClientOptions } from "./client.ts";
export { WsClient, type WcClientOptions } from "./ws.ts";
export type {
  FlowId,
  FlowRunId,
  UserId,
  NodeId,
  ErrorBody,
  RestResult,
} from "./types/common.ts";
export type {
  GetFlowOutputOutput,
  PartialConfig,
  SolanaActionConfig,
  StartFlowOutput,
  StartFlowParams,
  StartFlowSharedOutput,
  StartFlowSharedParams,
  StartFlowUnverifiedOutput,
  StartFlowUnverifiedParams,
  SubmitSignatureOutput,
  SubmitSignatureParams,
  ValuesConfig,
  StopFlowParams,
  StopFlowOutput,
  InitAuthOutput,
  ConfirmAuthOutput,
} from "./types/rest.ts";
export {
  type WsResponse,
  type AuthenticateRequest,
  type AuthenticateResponse,
  type SubscribeFlowRunEventsRequest,
  type SubscribeFlowRunEventsResponse,
  type SubscribeSignatureRequestsRequest,
  type SubscribeSignatureRequestsResponse,
  type SignatureRequestsEvent,
  type FlowRunEvent,
  type FlowRunEventEnum,
  type LogLevel,
  type FlowStart,
  type FlowError,
  type FlowLog,
  type FlowFinish,
  type NodeStart,
  type NodeError,
  type NodeOutput,
  type NodeLog,
  type NodeFinish,
  type ISignatureRequest,
  SignatureRequest,
} from "./types/ws.ts";

export function ed25519SignText(
  keypair: web3.Keypair,
  message: string
): Uint8Array {
  return naclSign(new TextEncoder().encode(message), keypair.secretKey);
}
