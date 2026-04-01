import type { ClientCore } from "../internal/core.ts";
import { flowRunTokenOutputSchema } from "@space-operator/contracts";
import { FlowRunHandle } from "../run_handle.ts";
import { performReadRequest, resolveReadAuthScope } from "../internal/read.ts";
import {
  normalizeFlowInputs,
  stableHash,
} from "../internal/transport/value.ts";
import { publicKeyAuth } from "../auth/mod.ts";
import type {
  DeploymentSpecifier,
  FlowRunId,
  PublicKeyProvider,
  ReadDeploymentParams,
  ReadResult,
  RequestOptions,
  StartDeploymentParams,
} from "../types.ts";

function deploymentQuery(specifier: DeploymentSpecifier): URLSearchParams {
  const query = new URLSearchParams();
  if ("id" in specifier) {
    query.set("id", specifier.id);
    return query;
  }

  query.set("flow", specifier.flow);
  query.set("tag", specifier.tag ?? "latest");
  return query;
}

function serializeStartDeploymentParams(params: StartDeploymentParams = {}) {
  return {
    ...(params.inputs !== undefined
      ? { inputs: normalizeFlowInputs(params.inputs) }
      : {}),
    ...(params.action_signer !== undefined
      ? { action_signer: params.action_signer }
      : {}),
  };
}

const MAX_READ_QUERY_BYTES = 1500;

function serializeReadDeploymentParams(params: ReadDeploymentParams = {}) {
  return {
    ...(params.inputs !== undefined
      ? { inputs: normalizeFlowInputs(params.inputs) }
      : {}),
    ...(params.skipCache !== undefined
      ? { skip_cache: params.skipCache }
      : {}),
  };
}

function readDeploymentRequestOptions(
  specifier: DeploymentSpecifier,
  params: ReadDeploymentParams,
) {
  const payload = serializeReadDeploymentParams(params);
  const encodedInputs = payload.inputs === undefined
    ? undefined
    : JSON.stringify(payload.inputs);
  if (
    (encodedInputs === undefined || encodedInputs.length <= MAX_READ_QUERY_BYTES) &&
    params.skipCache !== true
  ) {
    return {
      method: "GET",
      path: "/deployment/read",
      query: (() => {
        const query = deploymentQuery(specifier);
        if (encodedInputs !== undefined) {
          query.set("inputs", encodedInputs);
        }
        return query;
      })(),
    } as const;
  }

  return {
    method: "POST",
    path: "/deployment/read",
    query: deploymentQuery(specifier),
    body: payload,
  } as const;
}

export function createDeploymentsNamespace(core: ClientCore) {
  return {
    async start(
      specifier: DeploymentSpecifier,
      params: StartDeploymentParams = {},
      options: RequestOptions & { publicKey?: PublicKeyProvider } = {},
    ): Promise<FlowRunHandle> {
      const auth = options.publicKey
        ? publicKeyAuth(options.publicKey)
        : options.auth;
      const result = await core.requestContract(flowRunTokenOutputSchema, {
        method: "POST",
        path: "/deployment/start",
        auth,
        query: deploymentQuery(specifier),
        body: serializeStartDeploymentParams(params),
        headers: options.headers,
        signal: options.signal,
        retry: options.retry,
        timeoutMs: options.timeoutMs,
      }, "deployment start response");
      return new FlowRunHandle(core, result.flow_run_id, result.token);
    },

    async read(
      specifier: DeploymentSpecifier,
      params: ReadDeploymentParams = {},
      options: RequestOptions & { publicKey?: PublicKeyProvider } = {},
    ): Promise<ReadResult> {
      const auth = options.publicKey
        ? publicKeyAuth(options.publicKey)
        : options.auth;
      const normalized = params.inputs ? normalizeFlowInputs(params.inputs) : undefined;
      const authScope = await resolveReadAuthScope(core, {
        ...options,
        auth,
      });
      const scopeKey = "id" in specifier
        ? `id:${specifier.id}`
        : `flow:${specifier.flow}:${specifier.tag ?? "latest"}`;
      return await performReadRequest(core, {
        cacheKey: `deployment:read:${scopeKey}:${authScope}:${stableHash(normalized)}`,
        options: {
          ...options,
          auth,
        },
        request: {
          ...readDeploymentRequestOptions(specifier, params),
          auth,
          headers: options.headers,
          signal: options.signal,
          retry: options.retry,
          timeoutMs: options.timeoutMs,
        },
        skipCache: params.skipCache === true,
        subject: "deployment read response",
      });
    },

  };
}

export type DeploymentsNamespace = ReturnType<
  typeof createDeploymentsNamespace
>;
