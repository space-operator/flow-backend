import { Schema } from "effect";

// --- Branded ID Types ---

export const FlowId = Schema.String.pipe(Schema.brand("FlowId"));
export type FlowId = typeof FlowId.Type;

export const FlowRunId = Schema.String.pipe(Schema.brand("FlowRunId"));
export type FlowRunId = typeof FlowRunId.Type;

export const NodeId = Schema.String.pipe(Schema.brand("NodeId"));
export type NodeId = typeof NodeId.Type;

export const UserId = Schema.String.pipe(Schema.brand("UserId"));
export type UserId = typeof UserId.Type;

export const DeploymentId = Schema.String.pipe(Schema.brand("DeploymentId"));
export type DeploymentId = typeof DeploymentId.Type;

// --- Shared Schemas ---

export const ErrorBody = Schema.Struct({
  error: Schema.String,
});
export type ErrorBody = typeof ErrorBody.Type;

/**
 * Opaque schema for flow values (IValue from @space-operator/flow-lib).
 * These have a complex recursive structure; validation is left to the
 * flow-lib layer.  We treat them as `unknown` on the wire.
 */
export const IValueSchema = Schema.Unknown;
