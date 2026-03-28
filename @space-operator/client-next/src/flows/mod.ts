import { type IValue, Value } from "../deps.ts";
import {
  cloneFlowOutputSchema,
  flowRunStartOutputSchema,
  flowRunTokenOutputSchema,
  iValueSchema,
  signatureRequestSchema,
  successResponseSchema,
} from "@space-operator/contracts";
import type { ClientCore } from "../internal/core.ts";
import { normalizeFlowInputs } from "../internal/transport/value.ts";
import { FlowRunHandle } from "../run_handle.ts";
import { publicKeyAuth } from "../auth/mod.ts";
import type {
  CloneFlowOutput,
  DeploymentId,
  FlowId,
  FlowRunId,
  ISignatureRequest,
  PublicKeyProvider,
  RequestOptions,
  SignatureRequest,
  StartFlowParams,
  StartFlowSharedParams,
  StartFlowUnverifiedParams,
  StopFlowParams,
  SuccessResponse,
} from "../types.ts";
import { SignatureRequest as SignatureRequestModel } from "../types.ts";

function serializeStartFlowParams(params: StartFlowParams = {}) {
  return {
    ...(params.inputs !== undefined
      ? { inputs: normalizeFlowInputs(params.inputs) }
      : {}),
    ...(params.partial_config !== undefined
      ? { partial_config: params.partial_config }
      : {}),
    ...(params.environment !== undefined
      ? { environment: params.environment }
      : {}),
    ...(params.output_instructions !== undefined
      ? { output_instructions: params.output_instructions }
      : {}),
  };
}

function serializeStartFlowSharedParams(params: StartFlowSharedParams = {}) {
  return {
    ...(params.inputs !== undefined
      ? { inputs: normalizeFlowInputs(params.inputs) }
      : {}),
    ...(params.output_instructions !== undefined
      ? { output_instructions: params.output_instructions }
      : {}),
  };
}

function serializeStartFlowUnverifiedParams(
  params: StartFlowUnverifiedParams = {},
) {
  return {
    ...(params.inputs !== undefined
      ? { inputs: normalizeFlowInputs(params.inputs) }
      : {}),
    ...(params.output_instructions !== undefined
      ? { output_instructions: params.output_instructions }
      : {}),
    ...(params.action_identity !== undefined
      ? { action_identity: params.action_identity }
      : {}),
    ...(params.action_config !== undefined
      ? { action_config: params.action_config }
      : {}),
    ...(params.fees !== undefined ? { fees: params.fees } : {}),
  };
}

export function createFlowsNamespace(core: ClientCore) {
  return {
    async start(
      flowId: FlowId,
      params: StartFlowParams = {},
      options: RequestOptions = {},
    ): Promise<FlowRunHandle> {
      const result = await core.requestContract(flowRunStartOutputSchema, {
        method: "POST",
        path: `/flow/start/${flowId}`,
        auth: options.auth,
        body: serializeStartFlowParams(params),
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "flow start response");
      return new FlowRunHandle(
        core,
        result.flow_run_id,
        undefined,
        options.auth,
      );
    },

    async startShared(
      flowId: FlowId,
      params: StartFlowSharedParams = {},
      options: RequestOptions = {},
    ): Promise<FlowRunHandle> {
      const result = await core.requestContract(flowRunStartOutputSchema, {
        method: "POST",
        path: `/flow/start_shared/${flowId}`,
        auth: options.auth,
        body: serializeStartFlowSharedParams(params),
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "shared flow start response");
      return new FlowRunHandle(
        core,
        result.flow_run_id,
        undefined,
        options.auth,
      );
    },

    async startUnverified(
      flowId: FlowId,
      params: StartFlowUnverifiedParams = {},
      options: RequestOptions & { publicKey?: PublicKeyProvider } = {},
    ): Promise<FlowRunHandle> {
      const auth = options.publicKey
        ? publicKeyAuth(options.publicKey)
        : options.auth;
      const result = await core.requestContract(flowRunTokenOutputSchema, {
        method: "POST",
        path: `/flow/start_unverified/${flowId}`,
        auth,
        body: serializeStartFlowUnverifiedParams(params),
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "unverified flow start response");
      return new FlowRunHandle(core, result.flow_run_id, result.token);
    },

    async output(
      flowRunId: FlowRunId,
      options: RequestOptions = {},
    ): Promise<Value> {
      const value = await core.requestContract(iValueSchema, {
        method: "GET",
        path: `/flow/output/${flowRunId}`,
        auth: options.auth,
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "flow output response");
      return Value.fromJSON(value as IValue);
    },

    async signatureRequest(
      flowRunId: FlowRunId,
      options: RequestOptions = {},
    ): Promise<SignatureRequest> {
      const value = await core.requestContract(signatureRequestSchema, {
        method: "GET",
        path: `/flow/signature_request/${flowRunId}`,
        auth: options.auth,
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "signature request response");
      return new SignatureRequestModel(value as ISignatureRequest);
    },

    async stop(
      flowRunId: FlowRunId,
      params: StopFlowParams = {},
      options: RequestOptions = {},
    ): Promise<SuccessResponse> {
      return await core.requestContract(successResponseSchema, {
        method: "POST",
        path: `/flow/stop/${flowRunId}`,
        auth: options.auth,
        body: params,
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "flow stop response");
    },

    async deploy(
      flowId: FlowId,
      options: RequestOptions = {},
    ): Promise<DeploymentId> {
      const result = await core.requestJson<{ deployment_id: DeploymentId }>({
        method: "POST",
        path: `/flow/deploy/${flowId}`,
        auth: options.auth,
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      });
      return result.deployment_id;
    },

    async clone(
      flowId: FlowId,
      options: RequestOptions = {},
    ): Promise<CloneFlowOutput> {
      return await core.requestContract(cloneFlowOutputSchema, {
        method: "POST",
        path: `/flow/clone/${flowId}`,
        auth: options.auth,
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "flow clone response");
    },
  };
}

export type FlowsNamespace = ReturnType<typeof createFlowsNamespace>;
