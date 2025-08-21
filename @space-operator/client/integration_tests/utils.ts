import type { FlowRunId } from "../src/mod.ts";
import type { SupabaseClient } from "npm:@supabase/supabase-js@2";
import * as client from "../src/mod.ts";

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
    ...flowErrors.data.flatMap((row) => row.errors),
    ...nodeErrors.data.flatMap((row) => row.errors),
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
