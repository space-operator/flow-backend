import type { FlowError } from "../../types.ts";
import type { ZodIssue } from "@space-operator/contracts";

export class ClientError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = new.target.name;
  }
}

export class TransportError extends ClientError {}

export class TimeoutError extends ClientError {}

export class AbortError extends ClientError {}

export class WebSocketProtocolError extends ClientError {}

export class ContractValidationError extends ClientError {
  readonly issues: readonly ZodIssue[];
  readonly subject: string;

  constructor(subject: string, issues: readonly ZodIssue[]) {
    super(`invalid ${subject} contract`);
    this.subject = subject;
    this.issues = issues;
  }
}

export class FlowRunFailedError extends ClientError {
  readonly details: FlowError;
  readonly flowRunId: string;
  readonly time: string;

  constructor(details: FlowError) {
    super(details.error);
    this.details = details;
    this.flowRunId = details.flow_run_id;
    this.time = details.time;
  }
}

export interface ApiErrorContext {
  status: number;
  statusText: string;
  url: string;
  method: string;
  requestId?: string;
  body?: unknown;
}

export class ApiError extends ClientError {
  readonly status: number;
  readonly statusText: string;
  readonly url: string;
  readonly method: string;
  readonly requestId?: string;
  readonly body?: unknown;

  constructor(message: string, context: ApiErrorContext) {
    super(message);
    this.status = context.status;
    this.statusText = context.statusText;
    this.url = context.url;
    this.method = context.method;
    this.requestId = context.requestId;
    this.body = context.body;
  }
}
