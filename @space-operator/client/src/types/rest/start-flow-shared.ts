import { FlowRunId } from '../common.ts';
import { Value } from '../../deps.ts';

export interface StartFlowSharedParams {
  inputs: Record<string, Value>;
}

export interface StartFlowSharedOutput {
  flow_run_id: FlowRunId;
}
