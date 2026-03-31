import { z } from "zod";
export { z };

const nullToUndefined = <T extends z.ZodTypeAny>(schema: T) =>
  z.preprocess((value) => value === null ? undefined : value, schema.optional());

const timestampStringSchema = z.union([z.string(), z.number()]).transform((
  value,
) => typeof value === "number" ? new Date(value).toISOString() : value);

export const logLevelSchema = z.enum([
  "Trace",
  "Debug",
  "Info",
  "Warn",
  "Error",
]);

export const iValueSchema: z.ZodType = z.lazy(() =>
  z.union([
    z.object({ S: z.string() }).strict(),
    z.object({ D: z.string() }).strict(),
    z.object({ I: z.string() }).strict(),
    z.object({ U: z.string() }).strict(),
    z.object({ I1: z.string() }).strict(),
    z.object({ U1: z.string() }).strict(),
    z.object({ F: z.string() }).strict(),
    z.object({ B: z.boolean() }).strict(),
    z.object({ N: z.literal(0) }).strict(),
    z.object({ B3: z.string() }).strict(),
    z.object({ B6: z.string() }).strict(),
    z.object({ BY: z.string() }).strict(),
    z.object({ A: z.array(iValueSchema) }).strict(),
    z.object({ M: z.record(z.string(), iValueSchema) }).strict(),
  ])
);

export const jsonValueSchema: z.ZodType = z.lazy(() =>
  z.union([
    z.string(),
    z.number(),
    z.boolean(),
    z.null(),
    z.array(jsonValueSchema),
    z.record(z.string(), jsonValueSchema),
  ])
);

export const flowInputValueSchema: z.ZodType = z.lazy(() =>
  z.union([iValueSchema, jsonValueSchema])
);

export const flowInputsSchema = z.record(z.string(), flowInputValueSchema);

export const valuesConfigSchema = z.object({
  nodes: z.record(z.string(), z.string()),
  default_run_id: z.string().optional(),
}).strict();

export const partialConfigSchema = z.object({
  only_nodes: z.array(z.string()),
  values_config: valuesConfigSchema,
}).strict();

export const solanaActionConfigSchema = z.object({
  action_signer: z.string(),
  action_identity: z.string(),
}).strict();

export const startFlowParamsSchema = z.object({
  inputs: flowInputsSchema.optional(),
  partial_config: partialConfigSchema.optional(),
  environment: z.record(z.string(), z.string()).optional(),
  output_instructions: z.boolean().optional(),
}).strict();

export const startFlowSharedParamsSchema = z.object({
  inputs: flowInputsSchema.optional(),
  output_instructions: z.boolean().optional(),
}).strict();

export const startFlowUnverifiedParamsSchema = z.object({
  inputs: flowInputsSchema.optional(),
  output_instructions: z.boolean().optional(),
  action_identity: z.string().optional(),
  action_config: solanaActionConfigSchema.optional(),
  fees: z.array(z.tuple([z.string(), z.number()])).optional(),
}).strict();

export const stopFlowParamsSchema = z.object({
  timeout_millies: z.number().optional(),
  reason: z.string().optional(),
}).strict();

export const startDeploymentParamsSchema = z.object({
  inputs: flowInputsSchema.optional(),
  action_signer: z.string().optional(),
}).strict();

export const claimTokenOutputSchema = z.object({
  user_id: z.string(),
  access_token: z.string(),
  refresh_token: z.string(),
  expires_at: z.number(),
}).passthrough();

export const supabaseUserSchema = z.object({
  id: z.string(),
  aud: z.string(),
  created_at: z.string(),
  app_metadata: z.record(z.string(), z.unknown()),
  user_metadata: z.record(z.string(), z.unknown()),
}).passthrough();

export const supabaseSessionSchema = z.object({
  access_token: z.string(),
  refresh_token: z.string(),
  token_type: z.literal("bearer"),
  expires_at: nullToUndefined(z.number()),
  expires_in: z.number(),
  user: supabaseUserSchema,
}).passthrough();

export const confirmAuthOutputSchema = z.object({
  session: z.object({
    access_token: z.string(),
  }).passthrough(),
  new_user: z.boolean(),
}).passthrough();

export const successResponseSchema = z.object({
  success: z.literal(true),
}).passthrough();

export const flowRunStartOutputSchema = z.object({
  flow_run_id: z.string(),
}).passthrough();

export const flowRunTokenOutputSchema = flowRunStartOutputSchema.extend({
  token: z.string(),
}).passthrough();

export const cloneFlowOutputSchema = z.object({
  flow_id: z.string(),
  id_map: z.record(z.string(), z.string()),
}).passthrough();

export const apiKeyRecordSchema = z.object({
  key_hash: z.string(),
  trimmed_key: z.string(),
  name: z.string(),
  user_id: z.string(),
  created_at: z.string(),
}).passthrough();

export const createApiKeyOutputSchema = apiKeyRecordSchema.extend({
  full_key: z.string(),
}).passthrough();

export const apiKeyInfoOutputSchema = z.object({
  user_id: z.string(),
}).passthrough();

export const irohInfoSchema = z.object({
  node_id: z.string(),
  relay_url: z.string(),
  direct_addresses: z.array(z.string()),
}).passthrough();

export const serviceInfoOutputSchema = z.object({
  supabase_url: z.string(),
  anon_key: z.string(),
  iroh: irohInfoSchema,
  base_url: z.string(),
}).passthrough();

export const submitSignatureInputSchema = z.object({
  id: z.number(),
  signature: z.union([
    z.string(),
    z.instanceof(Uint8Array),
    z.instanceof(ArrayBuffer),
  ]),
  new_msg: z.union([
    z.string(),
    z.instanceof(Uint8Array),
    z.instanceof(ArrayBuffer),
  ]).optional(),
}).strict();

export const webSocketIdentitySchema = z.object({
  user_id: nullToUndefined(z.string()),
  pubkey: nullToUndefined(z.string()),
  flow_run_id: nullToUndefined(z.string()),
}).passthrough();

export const wsResponseSchema = <T extends z.ZodTypeAny>(schema: T) =>
  z.object({
    id: z.number(),
    Ok: schema.optional(),
    Err: z.string().optional(),
  }).passthrough();

export const signatureSchema = z.object({
  pubkey: z.string(),
  signature: z.string(),
}).passthrough();

export const signatureRequestSchema = z.object({
  id: z.number(),
  time: timestampStringSchema,
  pubkey: z.string(),
  message: z.string(),
  timeout: z.number(),
  flow_run_id: nullToUndefined(z.string()),
  signatures: nullToUndefined(z.array(signatureSchema)),
}).passthrough();

export const flowStartSchema = z.object({
  flow_run_id: z.string(),
  time: timestampStringSchema,
}).passthrough();

export const flowErrorSchema = z.object({
  flow_run_id: z.string(),
  time: timestampStringSchema,
  error: z.string(),
}).passthrough();

export const flowLogSchema = z.object({
  flow_run_id: z.string(),
  time: timestampStringSchema,
  level: logLevelSchema,
  module: nullToUndefined(z.string()),
  content: z.string(),
}).passthrough();

export const flowFinishWireSchema = z.object({
  flow_run_id: z.string(),
  time: timestampStringSchema,
  not_run: z.array(z.string()),
  output: iValueSchema,
}).passthrough();

export const nodeStartWireSchema = z.object({
  flow_run_id: z.string(),
  time: timestampStringSchema,
  node_id: z.string(),
  times: z.number(),
  input: iValueSchema,
}).passthrough();

export const nodeOutputWireSchema = z.object({
  flow_run_id: z.string(),
  time: timestampStringSchema,
  node_id: z.string(),
  times: z.number(),
  output: iValueSchema,
}).passthrough();

export const nodeErrorSchema = z.object({
  flow_run_id: z.string(),
  time: timestampStringSchema,
  node_id: z.string(),
  times: z.number(),
  error: z.string(),
}).passthrough();

export const nodeLogSchema = z.object({
  flow_run_id: z.string(),
  time: timestampStringSchema,
  node_id: z.string(),
  times: z.number(),
  level: logLevelSchema,
  module: nullToUndefined(z.string()),
  content: z.string(),
}).passthrough();

export const nodeFinishSchema = z.object({
  flow_run_id: z.string(),
  time: timestampStringSchema,
  node_id: z.string(),
  times: z.number(),
}).passthrough();

export const apiInputEventSchema = z.object({
  flow_run_id: z.string(),
  time: timestampStringSchema,
  url: z.string(),
}).passthrough();

export const flowRunWireEventSchemas = {
  FlowStart: z.object({
    stream_id: z.number(),
    event: z.literal("FlowStart"),
    data: flowStartSchema,
  }).passthrough(),
  FlowError: z.object({
    stream_id: z.number(),
    event: z.literal("FlowError"),
    data: flowErrorSchema,
  }).passthrough(),
  FlowFinish: z.object({
    stream_id: z.number(),
    event: z.literal("FlowFinish"),
    data: flowFinishWireSchema,
  }).passthrough(),
  FlowLog: z.object({
    stream_id: z.number(),
    event: z.literal("FlowLog"),
    data: flowLogSchema,
  }).passthrough(),
  NodeStart: z.object({
    stream_id: z.number(),
    event: z.literal("NodeStart"),
    data: nodeStartWireSchema,
  }).passthrough(),
  NodeOutput: z.object({
    stream_id: z.number(),
    event: z.literal("NodeOutput"),
    data: nodeOutputWireSchema,
  }).passthrough(),
  NodeError: z.object({
    stream_id: z.number(),
    event: z.literal("NodeError"),
    data: nodeErrorSchema,
  }).passthrough(),
  NodeFinish: z.object({
    stream_id: z.number(),
    event: z.literal("NodeFinish"),
    data: nodeFinishSchema,
  }).passthrough(),
  NodeLog: z.object({
    stream_id: z.number(),
    event: z.literal("NodeLog"),
    data: nodeLogSchema,
  }).passthrough(),
  SignatureRequest: z.object({
    stream_id: z.number(),
    event: z.literal("SignatureRequest"),
    data: signatureRequestSchema,
  }).passthrough(),
  ApiInput: z.object({
    stream_id: z.number(),
    event: z.literal("ApiInput"),
    data: apiInputEventSchema,
  }).passthrough(),
};

export const signatureRequestsEventSchema = z.object({
  stream_id: z.number(),
  event: z.literal("SignatureRequest"),
  data: signatureRequestSchema,
}).passthrough();

export const executeFlowResultEnvelopeSchema = z.object({
  flowRunId: z.string(),
  token: z.string().optional(),
  status: z.enum(["running", "success", "error", "pending_signature"]),
  output: z.record(z.string(), z.unknown()).optional(),
  error: z.string().optional(),
  signature_request: signatureRequestSchema.optional(),
  signing_url: z.string().optional(),
}).strict();

const jsonSchemaOpts = { unrepresentable: "any" as const };

export const clientJsonSchemas = {
  iValue: z.toJSONSchema(iValueSchema, jsonSchemaOpts),
  flowInputValue: z.toJSONSchema(flowInputValueSchema, jsonSchemaOpts),
  startFlowParams: z.toJSONSchema(startFlowParamsSchema, jsonSchemaOpts),
  startFlowSharedParams: z.toJSONSchema(startFlowSharedParamsSchema, jsonSchemaOpts),
  startFlowUnverifiedParams: z.toJSONSchema(startFlowUnverifiedParamsSchema, jsonSchemaOpts),
  startDeploymentParams: z.toJSONSchema(startDeploymentParamsSchema, jsonSchemaOpts),
  stopFlowParams: z.toJSONSchema(stopFlowParamsSchema, jsonSchemaOpts),
  signatureRequest: z.toJSONSchema(signatureRequestSchema, jsonSchemaOpts),
  executeFlowResultEnvelope: z.toJSONSchema(executeFlowResultEnvelopeSchema, jsonSchemaOpts),
};

export type IValueContract = z.infer<typeof iValueSchema>;
export type StartFlowParamsContract = z.infer<typeof startFlowParamsSchema>;
export type StartFlowSharedParamsContract = z.infer<
  typeof startFlowSharedParamsSchema
>;
export type StartFlowUnverifiedParamsContract = z.infer<
  typeof startFlowUnverifiedParamsSchema
>;
export type StopFlowParamsContract = z.infer<typeof stopFlowParamsSchema>;
export type StartDeploymentParamsContract = z.infer<
  typeof startDeploymentParamsSchema
>;
export type ClaimTokenOutputContract = z.infer<typeof claimTokenOutputSchema>;
export type ConfirmAuthOutputContract = z.infer<typeof confirmAuthOutputSchema>;
export type SuccessResponseContract = z.infer<typeof successResponseSchema>;
export type FlowRunStartOutputContract = z.infer<
  typeof flowRunStartOutputSchema
>;
export type FlowRunTokenOutputContract = z.infer<
  typeof flowRunTokenOutputSchema
>;
export type CloneFlowOutputContract = z.infer<typeof cloneFlowOutputSchema>;
export type CreateApiKeyOutputContract = z.infer<
  typeof createApiKeyOutputSchema
>;
export type ApiKeyInfoOutputContract = z.infer<typeof apiKeyInfoOutputSchema>;
export type ServiceInfoOutputContract = z.infer<typeof serviceInfoOutputSchema>;
export type SignatureRequestContract = z.infer<typeof signatureRequestSchema>;
export type ExecuteFlowResultEnvelopeContract = z.infer<
  typeof executeFlowResultEnvelopeSchema
>;
