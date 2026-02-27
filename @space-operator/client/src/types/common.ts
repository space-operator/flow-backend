export type FlowId = string;
export type DeploymentId = string;
export type FlowRunId = string;
export type NodeId = string;
export type UserId = string;
export interface ErrorBody {
  error: string;
}
export type RestResult<T> = T | ErrorBody;
