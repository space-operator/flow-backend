// --- External deps re-exported for consumer convenience ---
export { type IValue, Value } from "./deps.ts";

// --- Config ---
export {
  SpaceOperatorConfig,
  SpaceOperatorConfigFromEnv,
  makeConfig,
  type SpaceOperatorConfigShape,
} from "./Config.ts";

// --- Errors (Data.TaggedError) ---
export {
  AuthTokenError,
  HttpApiError,
  WsProtocolError,
  WsConnectionError,
  WsTimeoutError,
} from "./Errors.ts";

// --- Schemas: Common ---
export {
  FlowId,
  FlowRunId,
  NodeId,
  UserId,
  DeploymentId,
  ErrorBody,
  IValueSchema,
} from "./Schema/Common.ts";
export type {
  FlowId as FlowIdType,
  FlowRunId as FlowRunIdType,
  NodeId as NodeIdType,
  UserId as UserIdType,
  DeploymentId as DeploymentIdType,
} from "./Schema/Common.ts";

// --- Schemas: REST ---
export {
  InitAuthOutput,
  ConfirmAuthOutput,
  ClaimTokenOutput,
  StartFlowParams,
  StartFlowOutput,
  StartFlowSharedParams,
  StartFlowSharedOutput,
  SolanaActionConfig,
  StartFlowUnverifiedParams,
  StartFlowUnverifiedOutput,
  StopFlowParams,
  StopFlowOutput,
  SubmitSignatureParams,
  SubmitSignatureOutput,
  DeploymentSpecifier,
  formatDeploymentQuery,
  StartDeploymentParams,
  StartDeploymentOutput,
  DeployFlowOutput,
  ServerInfo,
  CreateApiKeyOutput,
  ApiKeyInfoOutput,
  KvWriteItemOutput,
  KvDeleteItemOutput,
  KvReadItemOutput,
  ExportOutput,
} from "./Schema/Rest.ts";

// --- Schemas: WebSocket ---
export {
  SignatureRequest,
  SignatureRequestSchema,
  type ISignatureRequest,
  type AuthenticateResponseOk,
  type FlowRunEvent,
  type FlowRunEventEnum,
  FlowStart,
  FlowError,
  FlowLog,
  FlowFinish,
  NodeStart,
  NodeOutput,
  NodeError,
  NodeLog,
  NodeFinish,
  ApiInput,
  type LogLevel,
  type SignatureRequestsEvent,
  makeAuthenticateRequest,
  makeSubscribeFlowRunEventsRequest,
  makeSubscribeSignatureRequestsRequest,
} from "./Schema/Ws.ts";

// --- Schemas: Wallet ---
export {
  UpsertWalletBody,
  UpsertWalletResponse,
} from "./Schema/Wallet.ts";

// --- HTTP Layer ---
export { SpaceHttpClient, SpaceHttpClientLive } from "./HttpApi.ts";

// --- Services ---
export { AuthService, AuthServiceLive } from "./AuthService.ts";
export { FlowService, FlowServiceLive } from "./FlowService.ts";
export { KvService, KvServiceLive } from "./KvService.ts";
export { ApiKeyService, ApiKeyServiceLive } from "./ApiKeyService.ts";
export { WalletService, WalletServiceLive } from "./WalletService.ts";
export {
  WsService,
  WsServiceLive,
  type WsConnectionState,
  type WsServiceOptions,
} from "./WsService.ts";

// --- Facade ---
export {
  SpaceOperatorLive,
  SpaceOperatorFromEnv,
  type SpaceOperatorServices,
} from "./SpaceOperator.ts";

// --- Convenience Functions ---
export {
  runFlow,
  runFlowWs,
  type RunFlowOptions,
  type RunFlowWsOptions,
  type RunFlowWsResult,
} from "./Convenience.ts";

// --- Supabase (generated, keep as-is) ---
export type { Database } from "./supabase.ts";
