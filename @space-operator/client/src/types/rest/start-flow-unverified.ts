import { FlowRunId } from '../common.ts';
import { IValue, Value } from '../../deps.ts';

export interface StartFlowUnverifiedParams {
  inputs?: Record<string, IValue>;
  output_instructions?: boolean;
}

export interface StartFlowUnverifiedOutput {
  flow_run_id: FlowRunId;
  token: string;
}
