import type { Value, IValue, SupabaseSession } from "../deps.ts";
import type { FlowId } from "../mod.ts";
import type { FlowRunId, NodeId } from "./common.ts";

export interface InitAuthOutput {
  msg: string;
}

export interface ConfirmAuthOutput {
  session: SupabaseSession;
  new_user: boolean;
}

export type GetFlowOutputOutput = Value;

export interface StartFlowSharedParams {
  inputs?: Record<string, Value>;
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
  fees?: Array<[string, number]>;
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
  inputs?: Record<string, Value>;
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

export interface IDeploymentSpecifier {
  id?: string;
  flow?: FlowId;
  tag?: string;
}

export class DeploymentSpecifier implements IDeploymentSpecifier {
  id?: string;
  flow?: FlowId;
  tag?: string;
  constructor(ctor: IDeploymentSpecifier) {
    this.id = ctor.id;
    this.flow = ctor.flow;
    this.tag = ctor.tag;
  }

  static Id(id: string): DeploymentSpecifier {
    return new DeploymentSpecifier({ id });
  }

  static Tag(flow: FlowId, tag: string): DeploymentSpecifier {
    return new DeploymentSpecifier({ flow, tag });
  }

  formatQuery(): string {
    const query = new URLSearchParams();
    if (this.id != null) {
      query.append("id", this.id);
    }
    if (this.flow != null) {
      query.append("flow", this.flow.toString());
      if (this.tag != null) {
        query.append("tag", this.tag);
      }
    }
    return query.toString();
  }
}

export interface StartDeploymentParams {
  inputs?: Record<string, IValue>;
  action_signer?: string;
}

export interface StartDeploymentOutput {
  flow_run_id: FlowRunId;
  token: string;
}
