import type { FlowRunId } from "../src/mod.ts";
import type { SupabaseClient } from "@supabase/supabase-js";
import type * as client from "../src/mod.ts";

export function getEnv(key: string): string {
  const env = Deno.env.get(key);
  if (env === undefined) throw new Error(`no env ${key}`);
  return env;
}

export function getUuidEnv(key: string): string {
  const value = getEnv(key);
  if (
    !/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i
      .test(value)
  ) {
    throw new Error(`${key} must be a UUID, got: ${value}`);
  }
  return value;
}

export async function checkNoErrors(
  sup: SupabaseClient<client.Database>,
  runId: FlowRunId,
) {
  const nodeErrors = await sup
    .from("node_run")
    .select("errors")
    .eq("flow_run_id", runId)
    .not("errors", "is", "null");
  if (nodeErrors.error) throw new Error(JSON.stringify(nodeErrors.error));
  const flowErrors = await sup
    .from("flow_run")
    .select("errors")
    .eq("id", runId)
    .not("errors", "is", "null");
  if (flowErrors.error) throw new Error(JSON.stringify(flowErrors.error));
  const errors = [
    ...flowErrors.data.flatMap((row) => ({ errors: row.errors, id: runId })),
    ...nodeErrors.data.flatMap((row) => ({ errors: row.errors, id: runId })),
  ];
  if (errors.length > 0) throw new Error(JSON.stringify(errors));
}

export async function checkNoFlowErrors(
  sup: SupabaseClient<client.Database>,
  runId: FlowRunId,
) {
  const flowErrors = await sup
    .from("flow_run")
    .select("errors")
    .eq("id", runId)
    .not("errors", "is", "null");
  if (flowErrors.error) throw new Error(JSON.stringify(flowErrors.error));
  const errors = [
    ...flowErrors.data.flatMap((row) => row.errors),
  ];
  if (errors.length > 0) throw new Error(JSON.stringify(errors));
}
