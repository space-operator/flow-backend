import { Schema } from "effect";
import {
  DeploymentId,
  FlowId,
  FlowRunId,
  IValueSchema,
  NodeId,
} from "./Common.ts";

// --- Auth ---

export const InitAuthOutput = Schema.Struct({
  msg: Schema.String,
});
export type InitAuthOutput = typeof InitAuthOutput.Type;

export const ConfirmAuthOutput = Schema.Struct({
  session: Schema.Unknown, // SupabaseSession — opaque external type
  new_user: Schema.Boolean,
});
export type ConfirmAuthOutput = typeof ConfirmAuthOutput.Type;

export const ClaimTokenOutput = Schema.Struct({
  access_token: Schema.String,
  refresh_token: Schema.String,
});
export type ClaimTokenOutput = typeof ClaimTokenOutput.Type;

// --- Flows ---

export const StartFlowParams = Schema.Struct({
  inputs: Schema.optional(
    Schema.Record({ key: Schema.String, value: IValueSchema }),
  ),
  partial_config: Schema.optional(
    Schema.Struct({
      only_nodes: Schema.Array(NodeId),
      values_config: Schema.Struct({
        nodes: Schema.Record({ key: NodeId, value: FlowRunId }),
        default_run_id: Schema.optional(FlowRunId),
      }),
    }),
  ),
  environment: Schema.optional(
    Schema.Record({ key: Schema.String, value: Schema.String }),
  ),
});
export type StartFlowParams = typeof StartFlowParams.Type;

export const StartFlowOutput = Schema.Struct({
  flow_run_id: FlowRunId,
});
export type StartFlowOutput = typeof StartFlowOutput.Type;

export const StartFlowSharedParams = Schema.Struct({
  inputs: Schema.optional(
    Schema.Record({ key: Schema.String, value: IValueSchema }),
  ),
});
export type StartFlowSharedParams = typeof StartFlowSharedParams.Type;

export const StartFlowSharedOutput = Schema.Struct({
  flow_run_id: FlowRunId,
});
export type StartFlowSharedOutput = typeof StartFlowSharedOutput.Type;

export const SolanaActionConfig = Schema.Struct({
  action_signer: Schema.String,
  action_identity: Schema.String,
});
export type SolanaActionConfig = typeof SolanaActionConfig.Type;

export const StartFlowUnverifiedParams = Schema.Struct({
  inputs: Schema.optional(
    Schema.Record({ key: Schema.String, value: IValueSchema }),
  ),
  output_instructions: Schema.optional(Schema.Boolean),
  action_identity: Schema.optional(Schema.String),
  action_config: Schema.optional(SolanaActionConfig),
  fees: Schema.optional(
    Schema.Array(Schema.Tuple(Schema.String, Schema.Number)),
  ),
});
export type StartFlowUnverifiedParams = typeof StartFlowUnverifiedParams.Type;

export const StartFlowUnverifiedOutput = Schema.Struct({
  flow_run_id: FlowRunId,
  token: Schema.String,
});
export type StartFlowUnverifiedOutput = typeof StartFlowUnverifiedOutput.Type;

// --- Stop Flow ---

export const StopFlowParams = Schema.Struct({
  /** Timeout in milliseconds before force-stopping. */
  timeout_millis: Schema.optional(Schema.Number),
});
export type StopFlowParams = typeof StopFlowParams.Type;

export const StopFlowOutput = Schema.Struct({
  success: Schema.Literal(true),
});
export type StopFlowOutput = typeof StopFlowOutput.Type;

// --- Flow Output ---

export const GetFlowOutputOutput = IValueSchema;

// --- Signature ---

export const SubmitSignatureParams = Schema.Struct({
  id: Schema.Number,
  signature: Schema.String,
  new_msg: Schema.optional(Schema.String),
});
export type SubmitSignatureParams = typeof SubmitSignatureParams.Type;

export const SubmitSignatureOutput = Schema.Struct({
  success: Schema.Literal(true),
});
export type SubmitSignatureOutput = typeof SubmitSignatureOutput.Type;

// --- Deployments ---

export const DeploymentSpecifier = Schema.Struct({
  id: Schema.optional(Schema.String),
  flow: Schema.optional(FlowId),
  tag: Schema.optional(Schema.String),
});
export type DeploymentSpecifier = typeof DeploymentSpecifier.Type;

/** Format a DeploymentSpecifier into URL query params. */
export function formatDeploymentQuery(spec: DeploymentSpecifier): string {
  const query = new URLSearchParams();
  if (spec.id != null) query.append("id", spec.id);
  if (spec.flow != null) {
    query.append("flow", spec.flow.toString());
    if (spec.tag != null) query.append("tag", spec.tag);
  }
  return query.toString();
}

export const StartDeploymentParams = Schema.Struct({
  inputs: Schema.optional(
    Schema.Record({ key: Schema.String, value: IValueSchema }),
  ),
  action_signer: Schema.optional(Schema.String),
});
export type StartDeploymentParams = typeof StartDeploymentParams.Type;

export const StartDeploymentOutput = Schema.Struct({
  flow_run_id: FlowRunId,
  token: Schema.String,
});
export type StartDeploymentOutput = typeof StartDeploymentOutput.Type;

export const DeployFlowOutput = Schema.Struct({
  deployment_id: DeploymentId,
});
export type DeployFlowOutput = typeof DeployFlowOutput.Type;

// --- Server Info ---

export const ServerInfo = Schema.Struct({
  supabase_url: Schema.String,
  anon_key: Schema.String,
  iroh: Schema.Struct({
    node_id: Schema.String,
    relay_url: Schema.String,
    direct_addresses: Schema.Array(Schema.String),
  }),
  base_url: Schema.String,
});
export type ServerInfo = typeof ServerInfo.Type;

// --- API Keys ---

export const CreateApiKeyOutput = Schema.Struct({
  full_key: Schema.String,
  key_hash: Schema.String,
  trimmed_key: Schema.String,
  name: Schema.String,
  user_id: Schema.String,
  created_at: Schema.String,
});
export type CreateApiKeyOutput = typeof CreateApiKeyOutput.Type;

export const ApiKeyInfoOutput = Schema.Struct({
  user_id: Schema.String,
});
export type ApiKeyInfoOutput = typeof ApiKeyInfoOutput.Type;

// --- KV Store ---

export const KvWriteItemOutput = Schema.Struct({
  old_value: Schema.NullOr(IValueSchema),
});
export type KvWriteItemOutput = typeof KvWriteItemOutput.Type;

export const KvDeleteItemOutput = Schema.Struct({
  old_value: IValueSchema,
});
export type KvDeleteItemOutput = typeof KvDeleteItemOutput.Type;

export const KvReadItemOutput = Schema.Struct({
  value: IValueSchema,
});
export type KvReadItemOutput = typeof KvReadItemOutput.Type;

// --- Data Import/Export ---

export const ExportOutput = Schema.Record({
  key: Schema.String,
  value: Schema.Unknown,
});
export type ExportOutput = typeof ExportOutput.Type;
