import type { ClientCore } from "../internal/core.ts";
import { flowRunTokenOutputSchema } from "@space-operator/contracts";
import { FlowRunHandle } from "../run_handle.ts";
import { normalizeFlowInputs } from "../internal/transport/value.ts";
import { publicKeyAuth } from "../auth/mod.ts";
import type {
  DeploymentSpecifier,
  FlowRunId,
  PublicKeyProvider,
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
  };
}

export type DeploymentsNamespace = ReturnType<
  typeof createDeploymentsNamespace
>;
