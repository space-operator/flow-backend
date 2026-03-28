import {
  type Attributes,
  type Span,
  SpanStatusCode,
  trace,
  type Tracer,
} from "@opentelemetry/api";
import type { ClientTelemetryOptions } from "../types.ts";

export interface ResolvedTelemetryConfig {
  tracer: Tracer;
  attributes: Attributes;
}

export function resolveTelemetryConfig(
  options: ClientTelemetryOptions | undefined,
): ResolvedTelemetryConfig {
  return {
    tracer: options?.tracer ??
      trace.getTracer(
        options?.tracerName ?? "@space-operator/client-next",
        options?.tracerVersion ?? "0.0.0",
      ),
    attributes: options?.attributes ?? {},
  };
}

function applyAttributes(span: Span, attributes: Attributes) {
  for (const [key, value] of Object.entries(attributes)) {
    if (value !== undefined) {
      span.setAttribute(key, value);
    }
  }
}

export async function withSpan<T>(
  telemetry: ResolvedTelemetryConfig,
  name: string,
  attributes: Attributes,
  run: (span: Span) => Promise<T>,
): Promise<T> {
  return await telemetry.tracer.startActiveSpan(name, async (span) => {
    applyAttributes(span, telemetry.attributes);
    applyAttributes(span, attributes);
    try {
      const result = await run(span);
      span.setStatus({ code: SpanStatusCode.OK });
      return result;
    } catch (error) {
      span.recordException(
        error instanceof Error ? error : new Error(String(error)),
      );
      span.setStatus({
        code: SpanStatusCode.ERROR,
        message: error instanceof Error ? error.message : String(error),
      });
      throw error;
    } finally {
      span.end();
    }
  });
}
