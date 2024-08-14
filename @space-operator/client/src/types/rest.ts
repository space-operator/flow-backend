import type { Value, IValue } from "../deps.ts";
import type { FlowRunId, NodeId } from "./common.ts";

export type GetFlowOutputOutput = Value;

export interface StartFlowSharedParams {
  inputs: Record<string, Value>;
}

export interface StartFlowSharedOutput {
  flow_run_id: FlowRunId;
}

export interface SolanaActionConfig {
  action_signer: string;
  action_identity: string;
}

export interface StartFlowUnverifiedParams {
  inputs?: Record<string, IValue>;
  output_instructions?: boolean;
  action_identity?: string;
  action_config?: SolanaActionConfig;
}

export interface StartFlowUnverifiedOutput {
  flow_run_id: FlowRunId;
  token: string;
}

export interface ValuesConfig {
  nodes: Record<NodeId, FlowRunId>;
  default_run_id?: FlowRunId;
}

export interface PartialConfig {
  only_nodes: Array<NodeId>;
  values_config: ValuesConfig;
}

export interface StartFlowParams {
  inputs: Record<string, Value>;
  partial_config?: PartialConfig;
  environment?: Record<string, string>;
}

export interface StartFlowOutput {
  flow_run_id: FlowRunId;
}

export interface SubmitSignatureParams {
  id: number;
  signature: string;
  new_msg?: string;
}

export interface SubmitSignatureOutput {
  success: true;
}

export interface StopFlowParams {
  timeout_millies?: number;
}

export interface StopFlowOutput {
  success: true;
}