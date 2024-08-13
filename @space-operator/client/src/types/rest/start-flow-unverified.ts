import { FlowRunId } from "../common.ts";
import { IValue, Value } from "../../deps.ts";

export interface SolanaActionConfig {
  action_signer: string;
  action_identity: string;
}

export interface StartFlowUnverifiedParams {
  inputs?: Record<string, IValue>;
  output_instructions?: boolean;
  action_config?: SolanaActionConfig;
}

export interface StartFlowUnverifiedOutput {
  flow_run_id: FlowRunId;
  token: string;
}
